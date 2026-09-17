"""Windows integration checks. Run after building/staging thumbgen; requires Blender and Pillow."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import time

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT/'target/release/thumbgen.exe'
SOURCE = ROOT/'examples/suzanne.blend'


def run(*args, env=None, success=True):
    result = subprocess.run([str(CLI), *map(str, args)], env=env,
                            capture_output=True, timeout=100)
    assert (result.returncode == 0) == success, result.stderr.decode(errors='replace')
    return result


def pixels(path):
    with Image.open(path) as im:
        return im.convert('RGBA').tobytes()


def without_preview(data):
    # This pinned fixture is an uncompressed, little-endian, 64-bit BLEND.
    assert data[:9] == b'BLENDER-v'
    output = bytearray(data[:12])
    cursor = 12
    removed = 0
    while cursor < len(data):
        code, size = struct.unpack_from('<4sI', data, cursor)
        block = data[cursor:cursor + 24 + size]
        assert len(block) == 24 + size
        if code == b'TEST':
            removed += 1
        else:
            output.extend(block)
        cursor += len(block)
    assert removed == 1
    return output


def main():
    original = hashlib.sha256(SOURCE.read_bytes()).digest()
    missing = dict(os.environ, MESHTHUMBS_BLENDER=str(ROOT/'target/no-blender.exe'))
    with tempfile.TemporaryDirectory(prefix='meshthumbs-blend-test-') as folder:
        work = Path(folder)
        model = work/'日本語 & $x.blend'
        shutil.copyfile(SOURCE, model)
        out = model.with_suffix('.png')
        stored = work/'stored.png'
        run(model, stored, 512, env=missing)
        for size in (256, 512, 1024):
            result = run('--export-png', size, model)
            assert b'used stored BLEND preview' not in result.stderr, result.stderr
            with Image.open(out) as im:
                assert im.size == (size, size)
            if size == 512:
                assert pixels(out) != pixels(stored), 'Geometry must differ from saved preview'
        result = run('--export-png', 512, model, env=missing)
        assert b'used stored BLEND preview' in result.stderr
        assert pixels(out) == pixels(stored)
        broken = dict(os.environ, MESHTHUMBS_BLENDER=str(Path(os.environ['WINDIR'])/'System32/where.exe'))
        result = run('--export-png', 512, model, env=broken)
        assert b'used stored BLEND preview' in result.stderr
        assert pixels(out) == pixels(stored)
        raw = work/'automatic.rgba'
        run(model, raw, 512, '--raw-rgba')
        assert raw.read_bytes() == pixels(stored), 'Automatic preview must not invoke Blender'

        # Append a large unused payload: preview extraction must remain bounded,
        # while non-BLEND mesh formats retain their 300 MiB input limit.
        large = work/'large.blend'
        shutil.copyfile(model, large)
        with large.open('r+b') as handle:
            handle.truncate(350 * 1024 * 1024)
        run(large, work/'large-preview.png', 512, env=missing)
        assert pixels(work/'large-preview.png') == pixels(stored)
        result = run('--export-png', 512, large, env=missing)
        assert b'used stored BLEND preview' in result.stderr
        assert pixels(large.with_suffix('.png')) == pixels(stored)
        large.rename(work/'large.glb')
        result = run(work/'large.glb', work/'rejected.png', 512, success=False)
        assert b'300 MiB' in result.stderr

        no_preview = work/'no-preview.blend'
        no_preview.write_bytes(without_preview(SOURCE.read_bytes()))
        run(no_preview, work/'absent.png', 512, success=False)
        result = run('--export-png', 512, no_preview)
        assert b'used stored BLEND preview' not in result.stderr
        no_preview.with_suffix('.png').write_bytes(b'old PNG')
        run('--export-png', 512, no_preview, env=missing, success=False)
        assert no_preview.with_suffix('.png').read_bytes() == b'old PNG'

        bad = work/'bad.blend'
        bad.write_bytes(b'invalid BLEND')
        bad.with_suffix('.png').write_bytes(b'old PNG')
        good = work/'later.ply'
        shutil.copyfile(ROOT/'examples/BoxVertexColors.ply', good)
        run('--export-png', 256, bad, good, success=False)
        assert bad.with_suffix('.png').read_bytes() == b'old PNG'
        assert good.with_suffix('.png').is_file()

        # Hold the real session mutex: a concurrent export must wait, then resume.
        kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        kernel.CreateMutexW.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_wchar_p]
        kernel.CreateMutexW.restype = ctypes.c_void_p
        kernel.ReleaseMutex.argtypes = kernel.CloseHandle.argtypes = [ctypes.c_void_p]
        mutex = kernel.CreateMutexW(None, True, 'Local\\MeshThumbs.BlenderExport')
        assert mutex and ctypes.get_last_error() != 183, 'Another export is already active'
        child = None
        try:
            out.write_bytes(b'waiting')
            child = subprocess.Popen([str(CLI), '--export-png', '512', str(model)],
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            time.sleep(1)
            assert child.poll() is None and out.read_bytes() == b'waiting'
            assert kernel.ReleaseMutex(mutex)
            stdout, stderr = child.communicate(timeout=70)
            assert child.returncode == 0 and b'used stored BLEND preview' not in stderr, (stdout, stderr)
        finally:
            kernel.CloseHandle(mutex)
            if child is not None and child.poll() is None:
                child.kill()
                child.wait()
        assert hashlib.sha256(model.read_bytes()).digest() == original
        assert not list(work.glob('.meshthumbs-*'))
        # Exercise evaluated modifiers, object material overrides, negative scale,
        # hidden geometry, and disabled file scripts using a small trusted fixture.
        blender = os.environ.get('MESHTHUMBS_BLENDER') or shutil.which('blender')
        if not blender:
            candidates = sorted((Path(os.environ['ProgramFiles'])/'Blender Foundation').glob('*/blender.exe'))
            assert candidates, 'Set MESHTHUMBS_BLENDER for fixture checks'
            blender = str(candidates[-1])
        def script(path, *args):
            subprocess.run([blender, '--background', '--factory-startup', '--disable-autoexec',
                            '--python-exit-code', '1', '--python', str(path), '--', *map(str, args)],
                           check=True, capture_output=True, timeout=40)
        script(ROOT/'crates/png_export/tests/blend_scene.py', work)
        scene = work/'evaluated.blend'
        before = scene.read_bytes()
        script(ROOT/'crates/png_export/src/export_blend.py', scene, work/'evaluated.glb')
        glb = (work/'evaluated.glb').read_bytes()
        length = struct.unpack_from('<I', glb, 12)[0]
        doc = json.loads(glb[20:20+length])
        assert len(doc['meshes']) == 1, 'Hidden sphere omitted; instances share geometry'
        assert len(doc['nodes']) == 2, 'Original and collection instance must both remain'
        assert sum(doc['accessors'][p['indices']]['count'] for m in doc['meshes'] for p in m['primitives']) == 72
        assert doc['materials'][0]['pbrMetallicRoughness']['baseColorFactor'] == [1, 0, 0, 1]
        assert [2, 3, -1] in [n['translation'] for n in doc['nodes']]
        assert not (work/'AUTOEXEC-RAN').exists()
        assert scene.read_bytes() == before
    assert hashlib.sha256(SOURCE.read_bytes()).digest() == original
    print('BLEND geometry, three sizes, Unicode, fallback, missing preview, batch failure, isolation and mutex checks passed.')


if __name__ == '__main__':
    main()
