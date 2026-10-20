const fs = require("fs");
const root = "/home/runner/work/lingua/lingua";
const spec = JSON.parse(fs.readFileSync(root + "/specs/google/discovery.json", "utf8"));
const schemas = spec.schemas;
const gen = fs.readFileSync(root + "/crates/lingua/src/providers/google/generated.rs", "utf8");
const enums = [...gen.matchAll(/^pub enum (\w+)/gm)].map((m) => m[1]);
const structs = [...gen.matchAll(/^pub struct (\w+)/gm)].map((m) => m[1]);
const pub = new Set(Object.keys(schemas).map((n) => (n.startsWith("V1main") ? n.slice(6) : n)));
console.log("enums not schema-derived:", enums.filter((e) => !pub.has(e)));
console.log("structs not schema-derived:", structs.filter((s) => !pub.has(s)));

// reachable set from GenerateContentRequest/Response
const reachable = new Set();
function visit(name) {
  if (reachable.has(name)) return;
  const s = schemas[name];
  if (!s) return;
  reachable.add(name);
  const refs = [];
  (function walk(v) {
    if (Array.isArray(v)) return v.forEach(walk);
    if (v && typeof v === "object") {
      if (typeof v.$ref === "string") refs.push(v.$ref);
      Object.values(v).forEach(walk);
    }
  })(s);
  refs.forEach(visit);
}
visit("GenerateContentRequest");
visit("GenerateContentResponse");

const inline = {};
for (const name of reachable) {
  const props = schemas[name].properties || {};
  for (const [pname, p] of Object.entries(props)) {
    if (p.enum && !p.$ref) (inline[pname] ||= []).push(name);
    const it = p.items;
    if (it && it.enum && !it.$ref) (inline[pname] ||= []).push(name + "[]");
  }
}
const pascal = (s) => s.charAt(0).toUpperCase() + s.slice(1);
console.log("inline enum properties (reachable):");
for (const k of Object.keys(inline).sort()) {
  const cand = pascal(k);
  console.log("  " + k + " -> " + cand + "  sources: " + inline[k].join(", ") +
    (pub.has(cand) ? "   *** COLLIDES WITH SCHEMA ID ***" : ""));
}
