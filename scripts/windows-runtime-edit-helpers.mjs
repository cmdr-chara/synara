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
      source = source.replace(
        makeCall,
        `makeEffectProcessCommand(${commands[index].command}, ${commands[index].args}, {`,
      );
    }

    const shellCount = (source.match(/^\s*shell: prepared\.shell,\n/gm) ?? []).length;
    const verbatimCount =
      (source.match(
        /^\s*\.\.\.\(prepared\.windowsVerbatimArguments \? \{ windowsVerbatimArguments: true \} : \{\}\),\n/gm,
      ) ?? []).length;
    if (shellCount !== commands.length || verbatimCount !== commands.length) {
      throw new Error(
        `${file}: expected ${commands.length} platform option pairs, found ${shellCount}/${verbatimCount}`,
      );
    }
    source = source.replace(/^\s*shell: prepared\.shell,\n/gm, "");
    source = source.replace(
      /^\s*\.\.\.\(prepared\.windowsVerbatimArguments \? \{ windowsVerbatimArguments: true \} : \{\}\),\n/gm,
      "",
    );

    if (!source.includes("ChildProcess.")) {
      source = source.replace(
        /import \{ ChildProcess, ChildProcessSpawner \} from "effect\/unstable\/process";/,
        'import { ChildProcessSpawner } from "effect/unstable/process";',
      );
    }
    return source;
  });
}
