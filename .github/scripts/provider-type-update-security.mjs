// Defense in depth for diagnostics. Isolation, rather than redaction, protects
// credentials from deliberately hostile code.
export function checkEnvironment(source = process.env) {
  const environment = { CARGO_TERM_COLOR: "never" };
  for (const name of [
    "PATH",
    "HOME",
    "TMPDIR",
    "TMP",
    "TEMP",
    "SystemRoot",
    "COMSPEC",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
    "LANG",
    "LC_ALL",
  ]) {
    if (source[name] !== undefined) environment[name] = source[name];
  }
  return environment;
}

export function credentialRedactor(environment = process.env) {
  const secrets = Object.entries(environment)
    .filter(
      ([name, value]) =>
        /key|token|secret|password|credential|authorization|cookie|dsn/i.test(
          name
        ) &&
        typeof value === "string" &&
        value.length >= 4
    )
    .flatMap(([, value]) => [
      value,
      encodeURIComponent(value),
      Buffer.from(value).toString("base64"),
    ])
    .sort((a, b) => b.length - a.length);
  return (text) => {
    let result = String(text);
    for (const secret of secrets)
      result = result.split(secret).join("[REDACTED]");
    return result;
  };
}

export function redactor(environment = process.env) {
  const redactCredentials = credentialRedactor(environment);
  return (text) => {
    return redactCredentials(text)
      .replace(
        /\b(?:sk-[A-Za-z0-9_-]{12,}|gh[pousr]_[A-Za-z0-9_]{12,}|github_pat_[A-Za-z0-9_]{12,})\b/g,
        "[REDACTED]"
      )
      .replace(/\b(Bearer\s+)[A-Za-z0-9_.~+\/-]+/gi, "$1[REDACTED]")
      .replace(
        /((?:api[_-]?key|auth[_-]?token|password|secret)\s*[=:]\s*)[^\s,;"']+/gi,
        "$1[REDACTED]"
      );
  };
}

export function redactValue(value, redact) {
  if (typeof value === "string") return redact(value);
  if (Array.isArray(value))
    return value.map((item) => redactValue(item, redact));
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [
        key,
        redactValue(item, redact),
      ])
    );
  }
  return value;
}
