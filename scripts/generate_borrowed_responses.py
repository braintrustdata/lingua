from pathlib import Path
import re
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[1]
source = (root / "crates/lingua/src/providers/openai/generated.rs").read_text()
names = [
    "InputItem",
    "InputItemContent",
    "ContentInputItemContentList",
    "SummaryText",
    "ChatCompletionRequestMessageExt",
    "ChatCompletionRequestMessageContentExt",
]
text_fields = {
    "InputItem": {"id", "encrypted_content"},
    "ContentInputItemContentList": {"text"},
    "SummaryText": {"text"},
}
blocks = []
for name in names:
    type_source = (
        (root / "crates/lingua/src/providers/openai/convert.rs").read_text()
        if name.endswith("Ext")
        else source
    )
    match = re.search(r"pub (struct|enum) " + name + r" \{", type_source)
    if match is None:
        raise RuntimeError(f"Missing canonical type {name}")
    start = type_source.rfind("#[derive(", 0, match.start())
    end = type_source.index("\n}", match.end()) + 2
    block = type_source[start:end]
    block = re.sub(r"^\s*///.*\n", "", block, flags=re.M)
    block = re.sub(r"^#\[ts\(.*\)\]\n|^\s*#\[ts\(.*\)\]\n", "", block, flags=re.M)
    block = block.replace(", TS)", ")")
    block = block.replace(f"pub {match.group(1)} {name} {{", f"pub {match.group(1)} {name}<'a> {{")
    for field in text_fields.get(name, set()):
        pattern = r"(    pub " + field + r": )(.*),"
        block, count = re.subn(
            pattern,
            lambda match: match[1] + match[2].replace("String", "BorrowText<'a>") + ",",
            block,
        )
        if count != 1:
            raise RuntimeError(f"Expected one canonical field {name}.{field}")
    if name in {"InputItemContent", "ChatCompletionRequestMessageContentExt"}:
        block = block.replace("String(String)", "String(BorrowText<'a>)")
    for dependency in names:
        block = re.sub(r"\b" + dependency + r"\b(?!<'a>)", dependency + "<'a>", block)
    lines = []
    for line in block.splitlines():
        if "<'a>" in line and not line.startswith("pub "):
            lines.append("    #[serde(borrow)]")
        lines.append(line)
    blocks.append("\n".join(lines))
output = "#![allow(dead_code)]\nuse super::generated::*;\nuse super::generated as openai;\nuse super::convert::{ChatCompletionRequestMessageContentPartExt, ChatCompletionRequestReasoning, ReasoningSignature};\nuse crate::processing::borrowed::BorrowText;\nuse crate::serde_json;\nuse serde::{Deserialize, Serialize};\n\n" + "\n\n".join(blocks) + "\n"
dest = root / "crates/lingua/src/providers/openai/borrowed_generated.rs"
if "--check" in sys.argv:
    with tempfile.NamedTemporaryFile(suffix=".rs") as temporary:
        Path(temporary.name).write_text(output)
        subprocess.run(["rustfmt", "--edition", "2021", temporary.name], check=True)
        if dest.read_text() != Path(temporary.name).read_text():
            raise RuntimeError("Borrowed Responses types are out of date")
else:
    dest.write_text(output)
    subprocess.run(["rustfmt", "--edition", "2021", str(dest)], check=True)
