# Regenerates the resumable-state save / load blocks of src/gpu_march.wgsl
# from the declarations (real values R take two words; rput / rget).
# usage: python3 tools/march_state.py src/gpu_march.wgsl
import re,sys
path=sys.argv[1]
src=open(path).read()
priv=[(m.group(1),m.group(2).strip()) for m in re.finditer(r'^var<private> (\w+): ([^;]+);',src,re.M)]
a=src.index('fn pixel(x: i32, y: i32, mode: u32) {')
b=src.index('    let so = state_slot;',a)
body=src[a:b]
loc=[]
for m in re.finditer(r'^    var (\w+)(?:: ([^=;]+))?(?: = ([^;]+))?;',body,re.M):
    n,t,init=m.group(1),m.group(2),m.group(3)
    if t is None:
        i=init.strip()
        if re.match(r'^-?\d+\.\d+$',i) or i.startswith('R('): t='R'
        elif re.match(r'^\d+u$',i) or i.startswith('P_'): t='u32'
        elif re.match(r'^-?\d+$',i): t='i32'
        elif i in('true','false') or '!=' in i or '==' in i: t='bool'
        elif i.startswith('vec3<R>'): t='vec3<R>'
        elif i.startswith('vec2<R>'): t='vec2<R>'
        elif i.startswith('mat3x3<R>'): t='mat3x3<R>'
        else: raise SystemExit('type? '+n+' '+i)
    loc.append((n,t.strip()))
skip={'dbg_r1','dbg_it1','dbg_rst','dbg_it2','dbg_rout','dbg_g','r_task_origin','r_task_dir','r_task','suspended','state_slot','cur_item','resuming','r_rec','r_keep','r_task_inside','dv'}
vars_=[v for v in priv+loc if v[0] not in skip]+[('guard','i32')]
sv=[];ld=[];off=1
def scalar(n,t):
    global off
    if t=='R':
        sv.append(f'rput(so + {off}u * ns, ns, {n});'); ld.append(f'{n} = rget(so + {off}u * ns, ns);'); off+=2
    elif t=='f32':
        sv.append(f'STATE[so + {off}u * ns] = bitcast<u32>({n});'); ld.append(f'{n} = bitcast<f32>(STATE[so + {off}u * ns]);'); off+=1
    elif t in('i32',):
        sv.append(f'STATE[so + {off}u * ns] = bitcast<u32>({n});'); ld.append(f'{n} = bitcast<i32>(STATE[so + {off}u * ns]);'); off+=1
    elif t=='u32':
        sv.append(f'STATE[so + {off}u * ns] = {n};'); ld.append(f'{n} = STATE[so + {off}u * ns];'); off+=1
    elif t=='bool':
        sv.append(f'STATE[so + {off}u * ns] = select(0u, 1u, {n});'); ld.append(f'{n} = (STATE[so + {off}u * ns] != 0u);'); off+=1
    else: raise SystemExit('scalar? '+t)
for n,t in vars_:
    m=re.match(r'vec([234])<(\w+)>$',t)
    if t in('R','f32','i32','u32','bool'): scalar(n,t)
    elif m:
        for i in range(int(m.group(1))): scalar(n+'.'+'xyzw'[i], m.group(2))
    elif t=='mat3x3<R>':
        for i in range(3):
            for jj in range(3): scalar(f'{n}[{i}].{"xyz"[jj]}','R')
    else:
        m=re.match(r'array<(.+), (\d+)>$',t); et,cnt=m.group(1),int(m.group(2))
        # the arrays of the ambient occlusion: only the pipelines that cast
        # its rays (MODE_KIND 2, 3) keep them (MODE_KIND is a pipeline
        # constant: the march saves and loads them not at all)
        guard = 'if (MODE_KIND == 2u || MODE_KIND == 3u) ' if n.startswith('ao_') else ''
        if et=='R':
            sv.append(guard+f'{{ for (var i = 0u; i < {cnt}u; i++) {{ rput(so + ({off}u + 2u * i) * ns, ns, {n}[i]); }} }}')
            ld.append(guard+f'{{ for (var i = 0u; i < {cnt}u; i++) {{ {n}[i] = rget(so + ({off}u + 2u * i) * ns, ns); }} }}'); off+=2*cnt
        elif et=='vec3<R>':
            sv.append(guard+f'{{ for (var i = 0u; i < {cnt}u; i++) {{ let q = so + ({off}u + 6u * i) * ns; rput(q, ns, {n}[i].x); rput(q + 2u * ns, ns, {n}[i].y); rput(q + 4u * ns, ns, {n}[i].z); }} }}')
            ld.append(guard+f'{{ for (var i = 0u; i < {cnt}u; i++) {{ let q = so + ({off}u + 6u * i) * ns; {n}[i] = vec3<R>(rget(q, ns), rget(q + 2u * ns, ns), rget(q + 4u * ns, ns)); }} }}'); off+=6*cnt
        else: raise SystemExit('array? '+t)
# replace the blocks
ls=src.index('    if (resuming) {\n',b)
le=src.index('        load_record(',ls)
ss=src.index('        if (turns >= P[TURN_BUDGET]) {\n',le)+len('        if (turns >= P[TURN_BUDGET]) {\n')
se=src.index('            save_record(',ss)
src=src[:ls]+'    if (resuming) {\n'+''.join('        '+l+'\n' for l in ld)+src[le:ss]+''.join('            '+l+'\n' for l in sv)+src[se:]
src=re.sub(r'const STATE_FIXED: u32 = \d+u;',f'const STATE_FIXED: u32 = {off}u;',src)
open(path,'w').write(src)
print('STATE_FIXED',off,'vars',len(vars_))
