"""Create a tiny original regression scene in a caller-owned temporary folder."""
import bpy
from pathlib import Path
import sys

work = Path(sys.argv[sys.argv.index('--') + 1])
bpy.ops.object.select_all(action='SELECT')
bpy.ops.object.delete(use_global=False)
bpy.ops.mesh.primitive_cube_add()
obj = bpy.context.object
obj.location = (2, 1, 3)
obj.scale = (-1, 2, 1)
blue = bpy.data.materials.new('DataBlue')
blue.diffuse_color = (0, 0, 1, 1)
red = bpy.data.materials.new('OverrideRed')
red.use_nodes = True
red.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value = (1, 0, 0, 1)
obj.data.materials.append(blue)
obj.material_slots[0].link = 'OBJECT'
obj.material_slots[0].material = red
obj.modifiers.new('Array', 'ARRAY').count = 2
collection = bpy.data.collections.new('Instanced Mesh')
bpy.context.scene.collection.children.link(collection)
for old in list(obj.users_collection):
    old.objects.unlink(obj)
collection.objects.link(obj)
instance = bpy.data.objects.new('Collection Instance', None)
instance.instance_type = 'COLLECTION'
instance.instance_collection = collection
instance.location = (10, 0, 0)
bpy.context.scene.collection.objects.link(instance)
bpy.ops.mesh.primitive_uv_sphere_add(location=(100, 100, 100))
bpy.context.object.hide_render = True
code = bpy.data.texts.new('must-not-run.py')
code.use_module = True
code.write(f'from pathlib import Path\nPath({str(work / "AUTOEXEC-RAN")!r}).write_text("bad")')
bpy.ops.wm.save_as_mainfile(filepath=str(work/'evaluated.blend'))
