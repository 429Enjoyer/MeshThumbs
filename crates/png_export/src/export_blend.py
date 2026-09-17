"""MIT, MeshThumbs contributors. Convert a visible evaluated scene; never save the source."""
import sys
from pathlib import Path

import bpy


def main():
    source, destination = sys.argv[sys.argv.index('--') + 1:]
    def phase(text):
        Path(destination).with_suffix('.phase.txt').write_text(text, encoding='utf-8')
    if bpy.app.version < (3, 6, 0):
        raise RuntimeError('Blender 3.6 or newer is required')
    phase('opening scene')
    bpy.ops.wm.open_mainfile(filepath=source, load_ui=False, use_scripts=False)
    # Convert at the saved frame, with viewport modifiers/visibility. Flatten
    # evaluated instances so armatures and modifiers survive without animation.
    graph = bpy.context.evaluated_depsgraph_get()
    meshes = []
    mesh_cache = {}
    triangles = 0
    phase('evaluating geometry')
    for instance in graph.object_instances:
        obj = instance.object
        if obj.type not in {'MESH', 'CURVE', 'SURFACE', 'FONT', 'META'}:
            continue
        if not instance.show_self or obj.hide_render:
            continue
        if not instance.is_instance and not obj.original.visible_get():
            continue
        if instance.is_instance and instance.parent and instance.parent.hide_render:
            continue
        key = obj.as_pointer()
        mesh = mesh_cache.get(key)
        if mesh is None:
            # Copy the existing evaluated mesh. new_from_object can re-evaluate
            # modifiers and crash Blender 4.0 on some instanced/multires objects.
            evaluated = obj.to_mesh()
            try:
                mesh = evaluated.copy() if evaluated is not None else None
            finally:
                obj.to_mesh_clear()
            if mesh is not None:
                mesh_cache[key] = mesh
        if mesh is None:
            continue
        # Mesh copies retain data materials, not object-linked overrides.
        for index, slot in enumerate(obj.material_slots):
            if slot.link == 'OBJECT' and index < len(mesh.materials):
                mesh.materials[index] = slot.material
        mesh.calc_loop_triangles()
        triangles += len(mesh.loop_triangles)
        if triangles > 10_000_000 or len(meshes) >= 100_000:
            raise RuntimeError('Scene exceeds thumbnail geometry limits')
        if mesh.loop_triangles:
            meshes.append((mesh, instance.matrix_world.copy()))
    if not meshes:
        raise RuntimeError('Scene has no visible mesh geometry')
    scene = bpy.data.scenes.new('MeshThumbs Export')
    for mesh, matrix in meshes:
        obj = bpy.data.objects.new('MeshThumbs Mesh', mesh)
        scene.collection.objects.link(obj)
        obj.matrix_world = matrix
    bpy.context.window.scene = scene
    phase('preparing textures')
    # Thumbnail textures are at most 1024px in MeshThumbs. Resize in memory before
    # GLB serialization to avoid exporting huge packed maps; never save the BLEND.
    materials = {material for mesh, _ in meshes for material in mesh.materials if material}
    images = set()
    def collect_images(tree, visited):
        if not tree or tree in visited:
            return
        visited.add(tree)
        for node in tree.nodes:
            if node.type == 'TEX_IMAGE' and node.image:
                images.add(node.image)
            elif node.type == 'GROUP':
                collect_images(node.node_tree, visited)
    for material in materials:
        collect_images(material.node_tree, set())
    for image in images:
        width, height = image.size
        if max(width, height) > 1024 and min(width, height) > 0:
            ratio = 1024 / max(width, height)
            image.scale(max(1, round(width * ratio)), max(1, round(height * ratio)))
    phase('writing GLB')
    bpy.ops.export_scene.gltf(
        filepath=destination, export_format='GLB', use_active_scene=True,
        export_animations=False, export_skins=False, export_morph=False,
        export_cameras=False, export_lights=False, export_extras=False,
    )
    if not Path(destination).is_file():
        raise RuntimeError('Blender produced no GLB')


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        # Return a bounded reason to the GUI without exposing a console window.
        destination = Path(sys.argv[sys.argv.index('--') + 2])
        destination.with_suffix('.error.txt').write_text(str(error)[:2000], encoding='utf-8')
        raise
