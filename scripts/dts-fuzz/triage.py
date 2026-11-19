import subprocess, os, sys, re, collections
cdir, bin = sys.argv[1], './fuzz_dts_asan'
env=dict(os.environ, ASAN_OPTIONS="detect_leaks=0:abort_on_error=0:exitcode=86:allocator_may_return_null=0:max_allocation_size_mb=4096")
sites=collections.Counter(); crashers=collections.defaultdict(list)
files=sorted(f for f in os.listdir(cdir) if f.endswith('.dts'))
for fn in files:
    p=os.path.join(cdir,fn)
    try: r=subprocess.run([bin,p],capture_output=True,timeout=5,env=env)
    except subprocess.TimeoutExpired: sites['HANG']+=1; crashers['HANG'].append(fn); continue
    out=(r.stdout+r.stderr).decode('latin1')
    if r.returncode==0: continue
    if 'ERROR: AddressSanitizer' in out:
        m=re.search(r'AddressSanitizer: ([\w-]+)',out); typ=m.group(1) if m else '?'
        if 'exceeds maximum supported size' in out: typ='alloc-size-too-big'
        frames=re.findall(r'#\d+ .*? in (\S+) .*?(DTS\w+\.(?:cpp|h):\d+)',out)
        site=None
        for fnname,loc in frames:
            if 'IMemBuffer' in fnname or 'CheckGuard' in fnname or 'DTSShapeAlloc' in loc: continue
            site=loc; break
        if not site and frames: site=frames[0][1]
        key=f'ASAN:{typ} @ {site}'
    elif 'allocation-size' in out or 'exceeds maximum' in out or 'bad_alloc' in out or '_M_default_append' in out or '_M_fill' in out:
        key='ALLOC-DoS (std::vector/new huge count)'
    elif 'ssert' in out:
        m=re.search(r'`([^`\']+)',out); key='assert:'+(m.group(1) if m else '?')
    else: key=f'other rc={r.returncode}'
    sites[key]+=1; 
    if len(crashers[key])<3: crashers[key].append(fn)
print(f'crashers: {sum(sites.values())} / {len(files)}')
for k,c in sites.most_common(): print(f'{c:4d}  {k}   e.g. {crashers[k][:2]}')
