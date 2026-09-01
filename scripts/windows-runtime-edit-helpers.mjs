import fs from "node:fs";

export function read(file) {
  return fs.readFileSync(file, "utf8");
}

export function write(file, content) {
  fs.writeFileSync(file, content.endsWith("\n") ? content : `${content}\n`);
}

export function replace(file, before, after, expected = 1) {
  const source = read(file);
  const count = source.split(before).length - 1;
  if (count !== expected) throw new Error(`${file}: expected ${expected} matches, found ${count}`);
  write(file, source.split(before).join(after));
}

export function replaceRegex(file, pattern, after, expected = 1) {
  const source = read(file);
  const flags = pattern.flags.includes("g") ? pattern.flags : `${pattern.flags}g`;
  const matcher = new RegExp(pattern.source, flags);
  const count = [...source.matchAll(matcher)].length;
  if (count !== expected) throw new Error(`${file}: expected ${expected} regex matches, found ${count}`);
  write(file, source.replace(matcher, after));
}

export function edit(file, transform) {
  const source = read(file);
  const result = transform(source);
  if (result === source) throw new Error(`${file}: transform made no changes`);
  write(file, result);
}

function removeFirstAfter(source, start, pattern, label, file) {
  const tail = source.slice(start);
  const flags = pattern.flags.replaceAll("g", "");
  const matcher = new RegExp(pattern.source, flags);
  const match = matcher.exec(tail);
  if (!match || match.index > 2_000) {
    throw new Error(`${file}: ${label} was not found in the migrated command block`);
  }
  const index = start + match.index;
  return source.slice(0, index) + source.slice(index + match[0].length);
}

export function migratePreparedEffectCommands(file, importPath, declarations, commands) {
  if (declarations.length !== commands.length) throw new Error(`${file}: migration spec mismatch`);
  edit(file, (initial) => {
    let source = initial.replace(
      /^import \{ prepareWindowsSafeProcess \} from "@synara\/shared\/windowsProcess";\n/m,
      "",
    );
    if (!source.includes(`from "${importPath}"`)) {
      const marker = /^(import .* from "effect\/unstable\/process";\n)/m;
      if (!marker.test(source)) throw new Error(`${file}: Effect process import not found`);
      source = source.replace(marker, `$1import { makeEffectProcessCommand } from "${importPath}";\n`);
    }

    for (let index = 0; index < declarations.length; index += 1) {
      const pattern = declarations[index];
      const flags = pattern.flags.includes("g") ? pattern.flags : `${pattern.flags}g`;
      const matcher = new RegExp(pattern.source, flags);
      const count = [...source.matchAll(matcher)].length;
      if (count !== 1) throw new Error(`${file}: declaration ${index} matched ${count} times`);
      source = source.replace(matcher, "");

      const makeCall = "ChildProcess.make(prepared.command, prepared.args, {";
      if (!source.includes(makeCall)) throw new Error(`${file}: make call ${index} not found`);
      const replacement = `makeEffectProcessCommand(${commands[index].command}, ${commands[index].args}, {`;
      source = source.replace(makeCall, replacement);
      const migratedStart = source.indexOf(replacement);
      source = removeFirstAfter(
        source,
        migratedStart,
        /^\s*shell: prepared\.shell,\n/m,
        "shell option",
        file,
      );
      source = removeFirstAfter(
        source,
        migratedStart,
        /^\s*\.\.\.\(prepared\.windowsVerbatimArguments \? \{ windowsVerbatimArguments: true \} : \{\}\),\n/m,
        "verbatim option",
        file,
      );
    }

    if (!source.includes("ChildProcess.")) {
      source = source.replace(
        /import \{ ChildProcess, ChildProcessSpawner \} from "effect\/unstable\/process";/,
        'import { ChildProcessSpawner } from "effect/unstable/process";',
      );
    }
    return source;
  });
}
