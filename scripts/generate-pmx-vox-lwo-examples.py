"""Recreate the original MIT Robot, Island, and Satellite regression assets.

Run from any directory with Python 3.10+ and Pillow. No downloaded models.
"""
from pathlib import Path
import math
import struct
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'target' / 'procedural-examples'
P = struct.pack


class Mesh:
    def __init__(self):
        self.faces = []

    def face(self, points, material):
        for i in range(1, len(points)-1):
            self.faces.append(([points[0], points[i], points[i+1]], material))

    def box(self, center, size, mat):
        for axis in range(3):
            u, v = (axis+1) % 3, (axis+2) % 3
            for sign in [-1, 1]:
                points = []
                for a, b in [(-1,-1),(1,-1),(1,1),(-1,1)]:
                    p = list(center)
                    p[axis] += sign*size[axis]/2
                    p[u] += a*size[u]/2
                    p[v] += b*size[v]/2
                    points.append(p)
                self.face(points if sign > 0 else points[::-1], mat)

    def sphere(self, center, scale, mat, rings=16, slices=32):
        def point(i, j):
            a, b = math.pi*i/rings, math.tau*j/slices
            return [center[0]+scale[0]*math.sin(a)*math.cos(b),
                    center[1]+scale[1]*math.cos(a),
                    center[2]+scale[2]*math.sin(a)*math.sin(b)]
        for i in range(rings):
            for j in range(slices):
                pts = [point(i,j),point(i,j+1),point(i+1,j+1),point(i+1,j)]
                if i == 0: pts = pts[1:]
                if i == rings-1: pts = pts[:3]
                self.face(pts,mat)


def normal(points):
    a,b,c=points
    u=[b[i]-a[i] for i in range(3)]; v=[c[i]-a[i] for i in range(3)]
    n=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
    length=math.sqrt(sum(x*x for x in n))
    return [x/length for x in n] if length > 1e-12 else [0,1,0]


def pmx(mesh, path, colors, version=2.0, encoding='utf-8'):
    def string(s):
        data=s.encode(encoding); return P('<i',len(data))+data
    # Four-byte indices and a single rest-pose root bone.
    data=bytearray(b'PMX '+P('<f',version)+bytes([8, int(encoding=='utf-8'),0,4,4,4,4,4,4]))
    data+=string('Robot')+string('Robot')+string('Copyright 2026 MeshThumbs contributors; MIT')+string('Original procedural model')
    faces=sorted(mesh.faces,key=lambda f:f[1])
    data+=P('<i',len(faces)*3)
    for points,mat in faces:
        n=normal(points)
        axis=max(range(3),key=lambda i:abs(n[i]))
        u,v=(axis+1)%3,(axis+2)%3
        lower=[min(p[i] for p in points) for i in range(3)]
        upper=[max(p[i] for p in points) for i in range(3)]
        for pos in points:
            uv=((pos[u]-lower[u])/max(upper[u]-lower[u],1e-8),
                1-(pos[v]-lower[v])/max(upper[v]-lower[v],1e-8))
            # PMX uses left-handed coordinates and clockwise front faces.
            data+=P('<8fBi f',pos[0],pos[1],-pos[2],n[0],n[1],-n[2],*uv,0,0,1.0)
    data+=P('<i',len(faces)*3)
    data+=b''.join(P('<i',i) for i in range(len(faces)*3))
    data+=P('<i',1)+string('assets/RobotBadge.png')
    data+=P('<i',len(colors))
    for i,color in enumerate(colors):
        count=sum(3 for _,mat in faces if mat==i)
        data+=string(f'Material{i}')*2+P('<4f',*color,1.0)+P('<7f',0,0,0,1,0,0,0)
        data+=P('<B5fiiBBBi',1,0,0,0,1,0,0 if i==len(colors)-1 else -1,-1,0,1,0,0)+P('<i',count)
    data+=P('<i',1)+string('root')*2+P('<3fiiH3f',0,0,0,-1,0,0,0,1,0)
    data+=P('<4i',0,0,0,0)
    if version > 2.0: data+=P('<i',0)
    path.write_bytes(data)


def lwo(mesh,path,colors):
    def string(s):
        data=s.encode()+b'\0'; return data+b'\0'*(len(data)%2)
    def chunk(id,data): return id+P('>I',len(data))+data+b'\0'*(len(data)%2)
    def sub(id,data): return id+P('>H',len(data))+data+b'\0'*(len(data)%2)
    faces=mesh.faces
    points=[p for tri,_ in faces for p in tri]
    assert len(points)<65280
    data=chunk(b'TAGS',b''.join(string(f'Material{i}') for i in range(len(colors))))
    data+=chunk(b'LAYR',P('>HH3f',0,0,0,0,0)+string('Satellite'))
    data+=chunk(b'PNTS',b''.join(P('>3f',p[0],p[1],-p[2]) for p in points))
    data+=chunk(b'POLS',b'FACE'+b''.join(P('>4H',3,i*3,i*3+2,i*3+1) for i in range(len(faces))))
    data+=chunk(b'PTAG',b'SURF'+b''.join(P('>HH',i,mat) for i,(_,mat) in enumerate(faces)))
    for i,color in enumerate(colors):
        surf=string(f'Material{i}')+string('')+sub(b'COLR',P('>3fH',*color,0))+sub(b'DIFF',P('>fH',1,0))
        data+=chunk(b'SURF',surf)
    path.write_bytes(b'FORM'+P('>I',len(data)+4)+b'LWO2'+data)


def vox(path):
    def chunk(id,data): return id+P('<ii',len(data),0)+data
    def dictionary(items):
        data=P('<i',len(items))
        for k,v in items.items():
            for s in [k,v]:
                raw=s.encode(); data+=P('<i',len(raw))+raw
        return data
    colors=[(66,138,173,255),(203,170,111,255),(72,150,97,255),(51,101,70,255),(112,78,51,255),(218,88,61,255),(239,219,166,255),(61,73,82,255)]
    cells={}
    # Floating island, stepped grass, a tiny cabin, path, and two trees.
    for x in range(48):
        for y in range(40):
            r=((x-24)/24)**2+((y-20)/20)**2
            if r<1:
                height=7+int(5*(1-r))
                for z in range(max(0,int(4*r)),height): cells[x,y,z]=3 if z==height-1 else 2
    def box(x0,y0,z0,x1,y1,z1,color):
        for x in range(x0,x1):
            for y in range(y0,y1):
                for z in range(z0,z1): cells[x,y,z]=color
    box(18,15,11,31,28,23,7)
    for z in range(7): box(16+z,13,23+z,33-z,30,24+z,6)
    box(22,14,11,26,15,19,5)
    box(19,14,17,21,15,20,1); box(28,14,17,30,15,20,1)
    for y in range(5,15): box(22,y,10,26,y+1,12,2)
    for x,y,height in [(10,23,31),(35,28,28)]:
        box(x-1,y-1,10,x+2,y+2,height-6,5)
        for z in range(height-15,height):
            radius=max(1,(height-z)//3)
            for a in range(x-radius,x+radius+1):
                for b in range(y-radius,y+radius+1):
                    if abs(a-x)+abs(b-y)<=radius+1: cells[a,b,z]=4 if (a+b)%3 else 3
    models=[((48,40,40),cells),((6,6,8),{(x,y,z):8 for x in range(6) for y in range(6) for z in range(8)})]
    data=b''
    for size,cells in models:
        data+=chunk(b'SIZE',P('<3i',*size))+chunk(b'XYZI',P('<i',len(cells))+b''.join(bytes((*pos,col)) for pos,col in sorted(cells.items())))
    palette=colors+[(0,0,0,255)]*(256-len(colors))
    data+=chunk(b'RGBA',b''.join(bytes(c) for c in palette))
    def trn(i,child,t,layer=0): return chunk(b'nTRN',P('<i',i)+dictionary({})+P('<4i',child,-1,layer,1)+dictionary({'_t':t,'_r':'4'}))
    def shape(i,model): return chunk(b'nSHP',P('<i',i)+dictionary({})+P('<2i',1,model)+dictionary({}))
    data+=trn(0,1,'0 0 0')
    data+=chunk(b'nGRP',P('<i',1)+dictionary({})+P('<3i',2,2,4))
    data+=trn(2,3,'0 0 0')+shape(3,0)+trn(4,5,'4 3 9')+shape(5,1)
    path.write_bytes(b'VOX '+P('<i',200)+b'MAIN'+P('<ii',0,len(data))+data)


def main():
    (OUT/'assets').mkdir(parents=True,exist_ok=True)
    badge=Image.new('RGB',(64,64),(230,241,243)); d=ImageDraw.Draw(badge)
    d.rounded_rectangle((8,8,56,56),radius=10,fill=(44,163,165))
    d.rectangle((28,18,36,46),fill=(247,207,72)); d.rectangle((18,28,46,36),fill=(247,207,72))
    badge.save(OUT/'assets/RobotBadge.png')
    robot=Mesh()
    colors=[(.13,.58,.63),(.87,.92,.94),(.055,.10,.14),(.95,.48,.29),(1,1,1)]
    robot.box((0,2.1,0),(1.5,1.5,.85),0)
    robot.sphere((0,3.55,0),(.98,.75,.65),1)
    robot.box((0,3.55,.70),(1.3,.54,.10),2)
    for x in [-.36,.36]: robot.sphere((x,3.6,.78),(.14,.14,.06),0)
    robot.box((0,3.30,.77),(.35,.055,.03),3)
    robot.box((0,2.15,.445),(.65,.65,.04),4)
    robot.box((0,4.30,0),(.09,.40,.09),2); robot.sphere((0,4.52,0),(.17,.17,.17),3)
    for s in [-1,1]:
        robot.sphere((s*.88,2.6,0),(.25,.25,.25),2)
        robot.box((s*1.07,2.05,0),(.34,1.0,.43),1)
        robot.sphere((s*1.1,1.47,0),(.27,.27,.27),3)
        robot.box((s*.43,.9,0),(.43,.90,.48),2)
        robot.box((s*.43,.35,.15),(.65,.4,.9),1)
    pmx(robot,OUT/'Robot.pmx',colors)
    satellite=Mesh()
    colors=[(.91,.66,.20),(.12,.25,.53),(.49,.66,.80),(.85,.89,.92),(.14,.17,.20)]
    satellite.box((0,0,0),(1.2,1.4,1.2),0)
    satellite.box((0,0,0),(5.9,.08,.08),3)
    for s in [-1,1]:
        satellite.box((s*2,0,0),(2.3,.08,1.8),3)
        for i in range(6):
            for j in range(4): satellite.box((s*2+(i-2.5)*.36,.055,(j-1.5)*.42),(.32,.04,.38),1 if (i+j)%2 else 2)
    satellite.sphere((0,.80,0),(.43,.32,.43),3)
    satellite.box((0,1.2,0),(.06,.8,.06),4)
    satellite.sphere((0,1.63,0),(.12,.12,.12),0)
    satellite.box((0,-.83,0),(.65,.25,.65),4)
    lwo(satellite,OUT/'Satellite.lwo',colors)
    vox(OUT/'Island.vox')
    print('Generated Robot.pmx, Island.vox, Satellite.lwo, and RobotBadge.png')


if __name__=='__main__': main()
