"""Recreate original MIT ASE, LXOB/LXO, LWS and DXF preview assets.

Python 3.10+ and Pillow; no downloaded geometry or textures.
"""
from pathlib import Path
import importlib.util
import math
import sys
from PIL import Image, ImageDraw

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'examples'
spec = importlib.util.spec_from_file_location('geometry', ROOT/'scripts/generate-pmx-vox-lwo-examples.py')
geometry = importlib.util.module_from_spec(spec); spec.loader.exec_module(geometry)
Mesh = geometry.Mesh


def zup(p): return (p[0], -p[2], p[1])


def ase(mesh, path, colors):
    lines=['*3DSMAX_ASCIIEXPORT 200','*COMMENT "Original MeshThumbs model; MIT"','*SCENE {', '*SCENE_FIRSTFRAME 0','*SCENE_LASTFRAME 0','*SCENE_FRAMESPEED 30','*SCENE_TICKSPERFRAME 160','}', '*MATERIAL_LIST {',f'*MATERIAL_COUNT {len(colors)}']
    for i,color in enumerate(colors):
        lines += [f'*MATERIAL {i} {{', f'*MATERIAL_NAME "Material{i}"','*MATERIAL_CLASS "Standard"','*MATERIAL_DIFFUSE '+' '.join(str(x/255) for x in color),'*MATERIAL_TRANSPARENCY 0']
        if i==3: lines += ['*MAP_DIFFUSE {','*MAP_CLASS "Bitmap"','*BITMAP "assets/ArcadeScreen.png"','*UVW_U_TILING 1','*UVW_V_TILING 1','}']
        lines += ['}']
    lines += ['}']
    for mat in range(len(colors)):
        faces=[p for p,m in mesh.faces if m==mat]; points=[p for f in faces for p in f]
        lines += ['*GEOMOBJECT {',f'*NODE_NAME "Part{mat}"','*NODE_TM {','*TM_ROW0 1 0 0','*TM_ROW1 0 1 0','*TM_ROW2 0 0 1','*TM_ROW3 0 0 0','*TM_POS 0 0 0','}',f'*MATERIAL_REF {mat}', '*MESH {','*TIMEVALUE 0',f'*MESH_NUMVERTEX {len(points)}',f'*MESH_NUMFACES {len(faces)}','*MESH_VERTEX_LIST {']
        for i,p in enumerate(points): lines += [f'*MESH_VERTEX {i} '+' '.join(f'{v:.6f}' for v in zup(p))]
        lines += ['}','*MESH_FACE_LIST {']
        for i in range(len(faces)): lines += [f'*MESH_FACE {i}: A: {i*3} B: {i*3+1} C: {i*3+2} AB: 1 BC: 1 CA: 1 *MESH_SMOOTHING 0 *MESH_MTLID 0']
        lines += ['}',f'*MESH_NUMTVERTEX {len(points)}','*MESH_TVERTLIST {']
        for i,p in enumerate(points):
            u=(p[0]+.66)/1.32; v=(p[1]-1.38)/.95
            lines += [f'*MESH_TVERT {i} {u:.6f} {v:.6f} 0']
        lines += ['}',f'*MESH_NUMTVFACES {len(faces)}','*MESH_TFACELIST {']
        for i in range(len(faces)): lines += [f'*MESH_TFACE {i} {i*3} {i*3+1} {i*3+2}']
        lines += ['}','}','}']
    path.write_text('\n'.join(lines)+'\n',encoding='utf8')


def lws(path):
    lines=['LWSC','3','FirstFrame 1','LastFrame 1','FramesPerSecond 30']
    for position,heading,scale in [((-2,0,0),-.4,.65),((2,.3,0),.4,.65),((0,2.3,.7),0,.55)]:
        lines += ['LoadObject Satellite.lwo','ObjectMotion','NumChannels 9']
        values=[*position,heading,0,0,scale,scale,scale]
        for i,value in enumerate(values):
            lines += [f'Channel {i}','{ Envelope','1',f'Key {value} 0 3 0 0 0 0 0 0','Behaviors 1 1','}']
    path.write_text('\n'.join(lines)+'\n',encoding='utf8')


def dxf(mesh,path):
    pairs=[(0,'SECTION'),(2,'HEADER'),(9,'$ACADVER'),(1,'AC1009'),(0,'ENDSEC'),(0,'SECTION'),(2,'ENTITIES')]
    for points,mat in mesh.faces:
        pairs += [(0,'3DFACE'),(8,'Pavilion'),(62,[8,145,33,7][mat])]
        for i,p in enumerate(points+[points[-1]]):
            for j,v in enumerate(zup(p)): pairs.append((10+10*j+i,f'{v:.7f}'))
    pairs += [(0,'ENDSEC'),(0,'EOF')]
    path.write_text(''.join(f'{code}\n{value}\n' for code,value in pairs),encoding='ascii')


def main():
    (OUT/'assets').mkdir(parents=True,exist_ok=True)
    im=Image.new('RGB',(256,192),(14,27,49)); draw=ImageDraw.Draw(im)
    for x in range(16,256,24): draw.line((x,0,x,192),fill=(22,49,68))
    for y in range(12,192,24): draw.line((0,y,256,y),fill=(22,49,68))
    draw.rectangle((35,34,220,155),outline=(52,197,197),width=5)
    draw.polygon([(107,65),(107,128),(155,97)],fill=(248,188,63))
    im.save(OUT/'assets/ArcadeScreen.png')
    arcade=Mesh(); arcade.box((0,1.25,0),(1.55,2.5,1.15),0)
    arcade.box((0,.13,0),(1.7,.26,1.3),1); arcade.box((0,2.62,.1),(1.7,.34,1.25),2)
    arcade.box((0,1.1,.68),(1.6,.17,.55),2)
    arcade.face([(-.66,1.38,.59),(.66,1.38,.59),(.66,2.33,.59),(-.66,2.33,.59)],3)
    arcade.box((-.3,1.27,.78),(.07,.22,.07),1); arcade.sphere((-.3,1.4,.78),(.12,.12,.12),2,6,12)
    for x in [.25,.48]: arcade.sphere((x,1.23,.78),(.08,.045,.08),2,6,12)
    ase(arcade,OUT/'Arcade.ase',[(52,82,116),(26,32,40),(236,160,49),(255,255,255)])
    camera=Mesh(); camera.box((0,.85,0),(2.2,1.3,.8),0); camera.box((-.7,1.6,0),(.7,.35,.7),1)
    camera.sphere((.22,.83,.55),(.65,.65,.45),1,14,32); camera.sphere((.22,.83,.85),(.48,.48,.25),2,14,32)
    camera.box((.8,1.58,0),(.3,.17,.3),2)
    geometry.lwo(camera,OUT/'Camera.lxo',[(.19,.23,.28),(.53,.58,.62),(.10,.43,.58)])
    data=bytearray((OUT/'Camera.lxo').read_bytes()); data[8:12]=b'LXOB'; (OUT/'Camera.lxo').write_bytes(data)
    lws(OUT/'Orbit.lws')
    pavilion=Mesh(); pavilion.box((0,.1,0),(3.7,.2,2.8),0)
    for x in [-1.45,1.45]:
        for z in [-.95,.95]: pavilion.box((x,1.05,z),(.22,1.8,.22),3)
    pavilion.box((0,1.92,0),(3.4,.15,2.5),2)
    for side in [-1,1]:
        a=[(-1.9,2,side*1.45),(1.9,2,side*1.45),(1.9,2.8,0),(-1.9,2.8,0)]
        pavilion.face(a if side>0 else a[::-1],1)
    dxf(pavilion,OUT/'Pavilion.dxf')
    print('Generated ASE, LXO, LWS, DXF and the original Arcade screen texture (MIT)')


if __name__=='__main__': main()
