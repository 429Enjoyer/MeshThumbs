"""Recreate original MIT SMD, MD2, MD3 and MD5MESH models and textures.

Requires Python 3.10+ and Pillow. No game assets or third-party models are used.
"""
from pathlib import Path
import importlib.util
import math
import struct
import sys
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'target' / 'procedural-examples'
P = struct.pack
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('preview_geometry', ROOT/'scripts/generate-pmx-vox-lwo-examples.py')
geometry = importlib.util.module_from_spec(spec)
spec.loader.exec_module(geometry)
Mesh, normal = geometry.Mesh, geometry.normal


def game(p):
    """Our Y-up geometry to the games' right-handed Z-up coordinates."""
    return (p[0], -p[2], p[1])


def fixed(s, n):
    data = s.encode('ascii'); assert len(data) < n
    return data.ljust(n, b'\0')


def atlas(name, colors, pcx=False):
    im = Image.new('RGB', (256, 32)); draw = ImageDraw.Draw(im)
    for i, c in enumerate(colors):
        a, b = i*256//len(colors), (i+1)*256//len(colors)
        draw.rectangle((a, 0, b-1, 31), fill=c)
        draw.line((a, 1, b-1, 1), fill=tuple(min(255, v+20) for v in c), width=2)
    path = OUT/'assets'/name
    if pcx: im = im.quantize(colors=256)
    im.save(path)


def uv(material, count):
    return ((material+.5)/count, .5)


def smd(mesh, path, texture, materials):
    lines = ['version 1', 'nodes', '0 "base" -1', '1 "turret" 0', 'end',
             'skeleton', 'time 0', '0 0 0 0 0 0 0', '1 0 0 1.5 0 0 0', 'end', 'triangles']
    for points, mat in mesh.faces:
        lines.append(texture)
        n = game(normal(points)); tex = uv(mat, materials)
        for p in points:
            weights = '2 0 0.25 1 0.75' if p[1] > 1.5 else '1 0 1'
            values = (*game(p), *n, *tex)
            lines.append('0 '+' '.join(f'{x:.7f}' for x in values)+' '+weights)
    path.write_text('\n'.join(lines+['end', '']), encoding='utf8', newline='\n')


def md2(mesh, path, texture, materials):
    faces = mesh.faces; count = len(faces)*3
    vertices = [game(p) for points, _ in faces for p in points]
    lo = [min(p[i] for p in vertices) for i in range(3)]
    hi = [max(p[i] for p in vertices) for i in range(3)]
    scale = [(hi[i]-lo[i])/255 or 1 for i in range(3)]
    # MD2's published normal-index encoding for the six cardinal directions.
    axes = [(52,(1,0,0)),(143,(-1,0,0)),(32,(0,1,0)),(104,(0,-1,0)),(5,(0,0,1)),(84,(0,0,-1))]
    data = bytearray()
    for points, mat in faces:
        n = game(normal(points)); ni = max(axes,key=lambda a:sum(n[i]*a[1][i] for i in range(3)))[0]
        for p in points:
            v = game(p)
            data += bytes([round((v[i]-lo[i])/scale[i]) for i in range(3)]+[ni])
    frames = b''
    for i in range(2):
        current_scale = [s*(1 if i==0 else 1.2) for s in scale]
        frames += P('<6f',*current_scale,*lo)+fixed(f'pose{i}',16)+data
    skin = fixed(texture,64)
    texcoords = b''.join(P('<2h',round(uv(mat,materials)[0]*256),16) for _,mat in faces for _ in range(3))
    tris = b''.join(P('<6H',i*3,i*3+2,i*3+1,i*3,i*3+2,i*3+1) for i in range(len(faces)))
    off_skin=68; off_uv=off_skin+len(skin); off_tri=off_uv+len(texcoords)
    off_frames=off_tri+len(tris); off_gl=off_frames+len(frames); end=off_gl+4
    header = b'IDP2'+P('<16i',8,256,32,40+count*4,1,count,count,len(faces),1,2,off_skin,off_uv,off_tri,off_frames,off_gl,end)
    path.write_bytes(header+skin+texcoords+tris+frames+P('<i',0))


def md3(mesh, path, texture, materials):
    surfaces = b''
    for mat in range(materials):
        faces = [points for points,m in mesh.faces if m==mat]; count=len(faces)*3
        tris = b''.join(P('<3i',i*3,i*3+2,i*3+1) for i in range(len(faces)))
        shader = fixed(texture,64)+P('<i',0)
        texcoords = P('<2f',*uv(mat,materials))*count
        frames = b''
        for frame in range(2):
            for points in faces:
                n = game(normal(points))
                lat = round(math.atan2(n[1],n[0])*255/math.tau)%256
                lng = round(math.acos(max(-1,min(1,n[2])))*255/math.tau)%256
                for p in points:
                    frames += P('<3hH',*[round(x*64*(1 if frame==0 else 1.2)) for x in game(p)],(lat<<8)|lng)
        off_tri=108; off_shader=off_tri+len(tris); off_uv=off_shader+len(shader)
        off_vertices=off_uv+len(texcoords); end=off_vertices+len(frames)
        header=b'IDP3'+fixed(f'part{mat}',64)+P('<10i',0,2,1,count,len(faces),off_tri,off_shader,off_uv,off_vertices,end)
        surfaces += header+tris+shader+texcoords+frames
    bounds = P('<10f16s',-10,-10,-10,10,10,10,0,0,0,20,b'pose0')*2
    offset=108+len(bounds)
    header=b'IDP3'+P('<i',15)+fixed(path.name,64)+P('<9i',0,2,0,materials,0,108,offset,offset,offset+len(surfaces))
    path.write_bytes(header+bounds+surfaces)


def md5(mesh, path, texture, materials):
    joints = [(0,0,0),(0,0,1.5)]
    lines = ['MD5Version 10', 'commandline "MeshThumbs original MIT example"', 'numJoints 2', 'numMeshes 1',
             'joints {', '"base" -1 ( 0 0 0 ) ( 0 0 0 )', '"upper" 0 ( 0 0 1.5 ) ( 0 0 0 )', '}',
             'mesh {', f'shader "{texture}"', f'numverts {len(mesh.faces)*3}']
    weights = []
    vertex = 0
    for points,mat in mesh.faces:
        for p in points:
            start=len(weights); influences=[(0,.25),(1,.75)] if p[1]>1.5 else [(0,1)]
            for bone,bias in influences:
                pos=game(p); local=tuple(pos[i]-joints[bone][i] for i in range(3))
                weights.append((bone,bias,local))
            u,v=uv(mat,materials)
            lines.append(f'vert {vertex} ( {u:.7f} {v:.7f} ) {start} {len(influences)}')
            vertex += 1
    lines.append(f'numtris {len(mesh.faces)}')
    for i in range(len(mesh.faces)): lines.append(f'tri {i} {i*3} {i*3+2} {i*3+1}')
    lines.append(f'numweights {len(weights)}')
    for i,(bone,bias,p) in enumerate(weights):
        lines.append(f'weight {i} {bone} {bias} ( '+ ' '.join(f'{x:.7f}' for x in p)+' )')
    path.write_text('\n'.join(lines+['}', '']),encoding='utf8',newline='\n')


def main():
    (OUT/'assets').mkdir(parents=True,exist_ok=True)
    crate=Mesh()
    crate.box((0,.8,0),(1.7,1.6,1.7),0)
    for y in [.16,1.44]: crate.box((0,y,0),(1.85,.22,1.85),1)
    for x in [-.77,.77]:
        for z in [-.77,.77]: crate.box((x,.8,z),(.20,1.5,.20),2)
    for y in [.4,.8,1.2]:
        crate.box((0,y,.86),(1.35,.025,.04),3); crate.box((.86,y,0),(.04,.025,1.35),3)
    crate.box((0,.8,.92),(.35,.5,.13),2)
    atlas('Crate.pcx',[(160,93,46),(104,61,32),(203,174,95),(58,39,27)],True)
    md2(crate,OUT/'Crate.md2','assets/Crate.pcx',4)

    drone=Mesh()
    drone.sphere((0,.45,0),(.75,.28,.65),0,8,16)
    drone.box((0,.17,.30),(.4,.3,.4),2)
    drone.sphere((0,.18,.52),(.16,.16,.12),3,8,12)
    for x in [-1.25,1.25]:
        for z in [-1.1,1.1]:
            drone.box((x/2,.45,z), (1.6,.13,.15),1)
            drone.box((0,.45,z/2),(.15,.13,1.25),1)
            drone.sphere((x,.58,z),(.18,.16,.18),2,6,12)
            drone.box((x,.76,z),(.9,.025,.10),1)
            drone.box((x,.77,z),(.10,.025,.9),1)
    atlas('Drone.png',[(224,229,230),(57,72,79),(238,114,51),(30,171,191)])
    md3(drone,OUT/'Drone.md3','assets/Drone.png',4)

    turret=Mesh()
    turret.box((0,.25,0),(2,.5,1.8),0)
    turret.box((0,.65,0),(1.25,.35,1.2),1)
    turret.sphere((0,1.25,0),(.85,.5,.65),0,10,20)
    turret.box((0,1.6,.30),(.85,.5,1.0),2)
    for x in [-.26,.26]:
        turret.box((x,1.6,1.05),(.16,.16,1.1),1)
        turret.box((x,1.6,1.64),(.23,.23,.12),2)
    turret.box((.55,1.55,.52),(.16,.25,.06),3)
    atlas('Turret.png',[(69,125,119),(41,52,58),(173,189,182),(245,176,53)])
    smd(turret,OUT/'Turret.smd','assets/Turret.png',4)

    mech=Mesh()
    mech.box((0,2.2,0),(1.5,1.45,.9),0)
    mech.box((0,3.2,.1),(.9,.65,.75),2)
    mech.box((0,3.28,.49),(.64,.12,.05),3)
    mech.box((0,2.5,.47),(.45,.5,.10),1)
    for sign in [-1,1]:
        mech.sphere((sign*.9,2.7,0),(.33,.33,.33),1,8,12)
        mech.box((sign*1.14,2.15,0),(.4,.9,.55),2)
        mech.box((sign*1.14,1.6,.2),(.5,.4,.8),0)
        mech.box((sign*.45,1.1,0),(.47,.9,.48),1)
        mech.box((sign*.45,.58,0),(.58,.6,.63),0)
        mech.box((sign*.45,.22,.2),(.75,.3,1.05),2)
    atlas('Mech_d.tga',[(198,88,56),(48,62,71),(197,207,209),(58,217,213)])
    md5(mech,OUT/'Mech.md5mesh','assets/Mech',4)
    print('Generated four game models and four original textures (MIT)')


if __name__=='__main__': main()
