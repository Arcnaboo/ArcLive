import struct
import json
import pathlib
import hashlib

path = pathlib.Path(r"C:\Program Files (x86)\Ableton\Live 7.0.18\Program\Live 7.0.18.exe")
data = path.read_bytes()
assert data[:2] == b"MZ"
e_lfanew = struct.unpack_from("<I", data, 0x3C)[0]
assert data[e_lfanew : e_lfanew + 4] == b"PE\0\0"
coff = e_lfanew + 4
machine, nsec, ts, _, _, optsize, chars = struct.unpack_from("<HHIIIHH", data, coff)
opt = coff + 20
magic = struct.unpack_from("<H", data, opt)[0]
if magic == 0x10B:
    entry = struct.unpack_from("<I", data, opt + 16)[0]
    imagebase = struct.unpack_from("<I", data, opt + 28)[0]
    subsystem = struct.unpack_from("<H", data, opt + 68)[0]
    dd_off = opt + 96
elif magic == 0x20B:
    entry = struct.unpack_from("<I", data, opt + 16)[0]
    imagebase = struct.unpack_from("<Q", data, opt + 24)[0]
    subsystem = struct.unpack_from("<H", data, opt + 68)[0]
    dd_off = opt + 112
else:
    raise SystemExit(f"bad magic {magic:#x}")

sec_off = opt + optsize
sections = []
for i in range(nsec):
    o = sec_off + i * 40
    name = data[o : o + 8].split(b"\0", 1)[0].decode("ascii", "replace")
    vsize, vaddr, rsize, rptr, _, _, _, chars_s = struct.unpack_from("<IIIIIIHH", data, o + 8)
    sections.append(
        {
            "name": name,
            "virt_size": vsize,
            "virt_addr": hex(vaddr),
            "raw_size": rsize,
            "raw_ptr": hex(rptr),
            "chars": hex(chars_s),
        }
    )


def rva_to_off(rva: int):
    for s in sections:
        va = int(s["virt_addr"], 16)
        vs = s["virt_size"]
        rs = s["raw_size"]
        rp = int(s["raw_ptr"], 16)
        if va <= rva < va + max(vs, rs):
            return rp + (rva - va)
    return None


imp_rva, imp_size = struct.unpack_from("<II", data, dd_off + 8)
imports = {}
if imp_rva:
    off = rva_to_off(imp_rva)
    while True:
        oft, td, _, name_rva, ft = struct.unpack_from("<IIIII", data, off)
        if oft == 0 and name_rva == 0 and ft == 0:
            break
        no = rva_to_off(name_rva)
        dll = data[no : data.find(b"\0", no)].decode("ascii", "replace")
        funcs = []
        thunk = rva_to_off(oft or ft)
        while True:
            hint_rva = struct.unpack_from("<I", data, thunk)[0]
            if hint_rva == 0:
                break
            if hint_rva & 0x80000000:
                funcs.append({"ordinal": hint_rva & 0xFFFF})
            else:
                ho = rva_to_off(hint_rva)
                name = data[ho + 2 : data.find(b"\0", ho + 2)].decode("ascii", "replace")
                funcs.append({"name": name, "hint": struct.unpack_from("<H", data, ho)[0]})
            thunk += 4
        imports[dll] = funcs
        off += 20

# export dir
exp_rva, exp_size = struct.unpack_from("<II", data, dd_off)
exports = []
if exp_rva:
    eo = rva_to_off(exp_rva)
    _, _, _, name_rva, base, nfunc, nnames, afunc, anames, aords = struct.unpack_from(
        "<IIIIIIiiii", data, eo
    )
    names_off = rva_to_off(anames)
    ords_off = rva_to_off(aords)
    for i in range(nnames):
        nrva = struct.unpack_from("<I", data, names_off + i * 4)[0]
        no = rva_to_off(nrva)
        n = data[no : data.find(b"\0", no)].decode("ascii", "replace")
        ordv = struct.unpack_from("<H", data, ords_off + i * 2)[0]
        exports.append({"name": n, "ordinal": base + ordv})

subsys_map = {1: "Native", 2: "GUI", 3: "CUI"}
out = {
    "file": str(path),
    "size": len(data),
    "sha256": hashlib.sha256(data).hexdigest(),
    "machine": hex(machine),
    "machine_name": "IMAGE_FILE_MACHINE_I386" if machine == 0x14C else hex(machine),
    "timestamp": ts,
    "characteristics": hex(chars),
    "pe_format": "PE32" if magic == 0x10B else "PE32+",
    "entry_point_rva": hex(entry),
    "image_base": hex(imagebase),
    "subsystem": subsys_map.get(subsystem, subsystem),
    "sections": sections,
    "export_count": len(exports),
    "exports": exports[:50],
    "import_dlls": sorted(imports),
    "imports": imports,
}
out_path = pathlib.Path(r"C:\Users\Arda\source\repos\arcc\analysis\live7\pe_header_imports.json")
out_path.write_text(json.dumps(out, indent=2))
print("wrote", out_path)
print("sections", len(sections), "dlls", len(imports), "exports", len(exports))
for d in sorted(imports):
    print(f"{d}: {len(imports[d])} imports")
