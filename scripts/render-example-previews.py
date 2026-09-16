"""Render the curated examples and build the README's 1920x1080 contact sheet."""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import subprocess

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / 'examples'
OUTPUT = ROOT / 'target/example-previews'
ROWS = [
    [('Suzanne.3ds', ''), ('Lantern.dae', ''), ('ToyCar.fbx', ''), ('BoomBox.glb', ''),
     ('ColorChain.3mf', ''), ('FlightHelmet.gltf', ''), ('Avocado.obj', ''), ('BoxVertexColors.ply', '')],
    [('WaterBottle.stl', ''), ('Avatar.vrm', ''), ('BarramundiFish.x3d', ''), ('SheenChair.off', ''),
     ('Sofa.usdz', 'USD · USDA · USDC · USDZ'), ('USB_Micro-B.wrl', 'WRL · VRML'),
     ('USB_Micro-B.step', 'STEP · STP'), ('Classroom.abc', 'Ogawa')],
    [('Gear.igs', 'IGS · IGES'), ('hello_mesh.3dm', 'Mesh'), ('Duplex_A_20110907.ifc', 'IFC2x3'),
     ('Airplane.pmx', 'PMX'), ('T-Rex.vox', 'MagicaVoxel'), ('boxuv.lwo', 'LightWave'),
     ('holy_grailref.smd', 'Reference mesh')],
    [('faerie.md2', 'First frame'), ('copter.md3', 'First frame · PCX skin'),
     ('Mech.md5mesh', 'Bind pose'), ('ThreeCubesGreen.ASE', ''), ('CrazyEngine.lxo', 'LXOB mesh'),
     ('move_x.lws', 'Referenced object'), ('wuson.dxf', 'Block inserts')],
]
EXTENSIONS = {'.'+name.lower() for row in ROWS for file, _ in row for name in [file.rsplit('.', 1)[1]]}
EXTENSIONS.update({'.usd', '.usda', '.usdc', '.vrml', '.stp', '.iges'})


def render(path):
    subprocess.run([str(ROOT/'target/release/thumbgen.exe'), str(path),
                    str(OUTPUT/(path.name+'.png')), '768'], check=True, timeout=60)
    with Image.open(OUTPUT/(path.name+'.png')) as image:
        assert image.size == (768, 768), path.name
        assert image.convert('RGBA').getchannel('A').getbbox(), path.name
    return path.name


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    files = sorted(p for p in EXAMPLES.iterdir() if p.suffix.lower() in EXTENSIONS)
    with ThreadPoolExecutor(max_workers=3) as pool:
        for name in pool.map(render, files):
            print('Rendered', name)
    canvas = Image.new('RGB', (1920, 1080), (25, 25, 25))
    draw = ImageDraw.Draw(canvas)
    font_path = 'C:/Windows/Fonts/segoeui.ttf'
    subtitle_font = ImageFont.truetype(font_path, 15)
    for row_index, row in enumerate(ROWS):
        top = 20 + row_index * 265
        for column, (name, subtitle) in enumerate(row):
            center = round(960 + (column - (len(row)-1)/2) * 236)
            with Image.open(OUTPUT/(name+'.png')) as source:
                thumb = source.convert('RGBA').resize((180, 180), Image.Resampling.LANCZOS)
            canvas.paste(thumb, (center-90, top), thumb)
            size = 20
            font = ImageFont.truetype(font_path, size)
            while draw.textlength(name, font=font) > 228:
                size -= 1
                font = ImageFont.truetype(font_path, size)
            draw.text((center, top+196), name, fill=(240, 240, 240), font=font, anchor='mt')
            draw.text((center, top+227), subtitle, fill=(155, 155, 155), font=subtitle_font, anchor='mt')
    destination = ROOT/'docs/images/meshthumbs-preview.png'
    canvas.save(destination, optimize=True)
    print(f'Updated {destination} from {len(files)} verified model files.')


if __name__ == '__main__':
    main()
