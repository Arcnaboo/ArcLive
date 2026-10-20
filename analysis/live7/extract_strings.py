"""Read-only ASCII/UTF-16LE string extraction with bounded keyword filters."""
from __future__ import annotations

import json
import pathlib
import re
from collections import defaultdict

TARGETS = {
    "exe": pathlib.Path(r"C:\Program Files (x86)\Ableton\Live 7.0.18\Program\Live 7.0.18.exe"),
    "engine": pathlib.Path(r"C:\Program Files (x86)\Ableton\Live 7.0.18\Program\Ableton Live Engine.dll"),
}

CATEGORIES = {
    "audio": [
        r"DirectSound",
        r"DSOUND",
        r"ASIO",
        r"waveOut",
        r"waveIn",
        r"sample\s*rate",
        r"buffer\s*size",
        r"audio\s*device",
        r"AudioEngine",
        r"Live Engine",
        r"Ableton Live Engine",
        r"AudioRipper",
        r"dsound",
        r"MME",
        r"WASAPI",
        r"CoreAudio",
    ],
    "midi": [
        r"midiIn",
        r"midiOut",
        r"MIDI",
        r"midi",
        r"MMC",
        r"MTC",
        r"timecode",
        r"clock\s*sync",
        r"WinMM",
        r"midiStream",
    ],
    "vst": [
        r"VSTPluginMain",
        r"AEffect",
        r"audioMaster",
        r"effOpen",
        r"effClose",
        r"effProcess",
        r"VST",
        r"\.dll",
        r"Plug-?in",
        r"plugin",
        r"Dispatcher",
    ],
    "serialization": [
        r"\.als",
        r"Ableton",
        r"gzip",
        r"zlib",
        r"deflate",
        r"inflate",
        r"xml",
        r"XML",
        r"Song",
        r"Preset",
        r"LiveSet",
        r"\.adv",
        r"\.adg",
        r"\.alc",
        r"\.asd",
        r"Serialize",
        r"Archive",
    ],
    "modules": [
        r"Ableton Live Engine",
        r"OceanM",
        r"Analog\.dll",
        r"Electric\.dll",
        r"Tension\.dll",
        r"LoadLibrary",
        r"GetProcAddress",
        r"FreeLibrary",
    ],
}


def extract_ascii(data: bytes, min_len: int = 4):
    for m in re.finditer(rb"[\x20-\x7e]{%d,}" % min_len, data):
        yield m.start(), m.group().decode("ascii")


def extract_utf16le(data: bytes, min_len: int = 4):
    # crude UTF-16LE printable runs
    i = 0
    n = len(data)
    while i + 2 <= n:
        start = i
        chars = []
        while i + 2 <= n:
            c = data[i] | (data[i + 1] << 8)
            if 0x20 <= c <= 0x7E:
                chars.append(chr(c))
                i += 2
            else:
                break
        if len(chars) >= min_len:
            yield start, "".join(chars)
        i = max(i + 2, start + 2)


def classify(s: str):
    hits = []
    for cat, pats in CATEGORIES.items():
        for p in pats:
            if re.search(p, s, re.IGNORECASE):
                hits.append(cat)
                break
    return hits


def scan(label: str, path: pathlib.Path):
    data = path.read_bytes()
    found = defaultdict(list)
    seen = set()
    for enc, iterator in (("ascii", extract_ascii(data)), ("utf16le", extract_utf16le(data))):
        for off, s in iterator:
            cats = classify(s)
            if not cats:
                continue
            key = (enc, s)
            if key in seen:
                continue
            seen.add(key)
            # keep short list per category
            for cat in cats:
                if len(found[cat]) >= 80:
                    continue
                found[cat].append({"offset": hex(off), "encoding": enc, "value": s[:240]})
    return {
        "label": label,
        "path": str(path),
        "size": len(data),
        "category_counts": {k: len(v) for k, v in found.items()},
        "strings": dict(found),
    }


def main():
    results = []
    for label, path in TARGETS.items():
        print("scanning", label, path)
        results.append(scan(label, path))
    out = pathlib.Path(r"C:\Users\Arda\source\repos\arcc\analysis\live7\string_anchors.json")
    out.write_text(json.dumps(results, indent=2))
    print("wrote", out)
    for r in results:
        print(r["label"], r["category_counts"])


if __name__ == "__main__":
    main()
