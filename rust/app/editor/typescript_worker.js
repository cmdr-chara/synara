// Synara editor intelligence worker — JavaScript port of upstream
// apps/server/src/editor/typescriptService.ts. One process per workspace;
// speaks JSON lines on stdio: in {id, input}, out {id, result} or {id, error}.
// Runs only the compiler service: no plugins, emit, shell, or disk writes.
"use strict";

const path = require("node:path");
const fs = require("node:fs");
const readline = require("node:readline");

const MAX_PROJECT_FILES = 10_000;
const MAX_RESULT_ITEMS = 2_000;

function loadTypeScript() {
  const candidates = [];
  if (process.env.SYNARA_TYPESCRIPT_LIB) candidates.push(process.env.SYNARA_TYPESCRIPT_LIB);
  // Walk up from the workspace for a project-local install first.
  let dir = process.env.SYNARA_TS_WORKSPACE || process.cwd();
  for (let i = 0; i < 12; i += 1) {
    candidates.push(path.join(dir, "node_modules", "typescript", "lib", "typescript.js"));
    const bunStore = path.join(dir, "node_modules", ".bun");
    if (fs.existsSync(bunStore)) {
      candidates.push(
        ...fs
          .readdirSync(bunStore, { withFileTypes: false })
          .filter((name) => /^typescript@\d/.test(name) && !name.includes("+"))
          .sort()
          .reverse()
          .map((name) =>
            path.join(bunStore, name, "node_modules", "typescript", "lib", "typescript.js"),
          ),
      );
    }
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  for (const candidate of candidates) {
    try {
      if (candidate && fs.existsSync(candidate)) return require(candidate);
    } catch {
      /* try the next candidate */
    }
  }
  try {
    return require("typescript");
  } catch {
    return null;
  }
}

const ts = loadTypeScript();
if (!ts) {
  process.stdout.write(
    JSON.stringify({ id: "init", error: "typescript package not found" }) + "\n",
  );
  // Keep reading so the parent sees a clean exit rather than a crash loop.
  process.stdin.resume();
  setTimeout(() => process.exit(3), 250);
} else {
  const display = (parts) => ts.displayPartsToString(parts ? [...parts] : undefined);

  function isInside(root, file) {
    const rel = path.relative(root, file);
    return rel === "" || (!rel.startsWith("..") && !path.isAbsolute(rel));
  }

  function createService(workspaceRoot) {
    const root = (ts.sys.realpath && ts.sys.realpath(workspaceRoot)) || path.resolve(workspaceRoot);
    const libraryRoot = path.dirname(
      require.resolve ? path.dirname(path.resolve(__filename)) : __filename,
    );
    // The library root is wherever the loaded typescript package keeps lib.d.ts.
    const tsLibRoot = path.join(path.dirname(require.main ? require.main.filename : __filename));
    let libRoot = null;
    // Discover lib.d.ts next to the typescript.js that was loaded.
    for (const candidate of [process.env.SYNARA_TYPESCRIPT_LIB]) {
      if (candidate && fs.existsSync(candidate)) {
        libRoot = path.dirname(candidate);
        break;
      }
    }
    if (!libRoot) {
      // Loaded via require() — find it through the module cache.
      const key = Object.keys(require.cache).find((k) => k.endsWith("typescript.js"));
      libRoot = key ? path.dirname(key) : __dirname;
    }
    libRoot = (ts.sys.realpath && ts.sys.realpath(libRoot)) || libRoot;

    let documents = new Map();
    let nextRevision = 0;
    let projectDirectory = root;
    let lastConfigRefresh = 0;
    let projectVersion = 0;
    let files = [];
    let options = {};
    let configuredFiles = [];
    let configuredOptions = {};
    let configStamp = "";
    let inferredJavaScript = false;
    const inferredOptions = {
      allowJs: true,
      checkJs: true,
      target: ts.ScriptTarget.ESNext,
      module: ts.ModuleKind.ESNext,
      moduleResolution: ts.ModuleResolutionKind.Bundler,
      jsx: ts.JsxEmit.ReactJSX,
      allowImportingTsExtensions: true,
      noEmit: true,
      plugins: [],
    };

    const allowed = (file) => {
      const absolute = path.resolve(file);
      if (!isInside(root, absolute) && !isInside(libRoot, absolute)) return false;
      try {
        const real = (ts.sys.realpath && ts.sys.realpath(absolute)) || absolute;
        return isInside(root, real) || isInside(libRoot, real);
      } catch {
        return false;
      }
    };
    const resolve = (relative) => {
      if (path.isAbsolute(relative) || relative.includes("\0"))
        throw new Error("Expected a workspace-relative path.");
      const file = path.resolve(root, relative);
      if (!isInside(root, file) || !allowed(file))
        throw new Error("Language request escaped its workspace.");
      return file;
    };
    const readFile = (file) => {
      if (!allowed(file)) return undefined;
      const contents = documents.get(path.resolve(file))?.contents ?? ts.sys.readFile(file);
      return contents !== undefined && contents.length <= 2_000_000 ? contents : undefined;
    };
    const fileExists = (file) =>
      allowed(file) && (documents.has(path.resolve(file)) || ts.sys.fileExists(file));
    const readDirectory = (dir, extensions, excludes, includes, depth) =>
      allowed(dir)
        ? ts.sys
            .readDirectory(dir, extensions, excludes, includes, depth)
            .filter(allowed)
            .slice(0, MAX_PROJECT_FILES + 1)
        : [];

    const parseConfig = (configFile, stamp) => {
      const baseDirectory = configFile ? path.dirname(configFile) : root;
      const config = configFile ? ts.readConfigFile(configFile, readFile) : undefined;
      if (config?.error)
        throw new Error(ts.flattenDiagnosticMessageText(config.error.messageText, "\n"));
      const parsed = ts.parseJsonConfigFileContent(
        config?.config ?? {
          compilerOptions: {
            allowJs: true,
            checkJs: true,
            target: "ESNext",
            module: "ESNext",
            moduleResolution: "Bundler",
            jsx: "react-jsx",
            allowImportingTsExtensions: true,
          },
          exclude: ["node_modules", ".git", "dist", "build"],
        },
        { ...ts.sys, readFile, fileExists, readDirectory },
        baseDirectory,
        { noEmit: true },
        configFile,
      );
      if (parsed.fileNames.length > MAX_PROJECT_FILES)
        throw new Error(
          "This project exceeds the language service's 10,000-file limit. Open a narrower workspace.",
        );
      configuredFiles = parsed.fileNames.filter(allowed);
      configuredOptions = { ...parsed.options, noEmit: true, plugins: [] };
      projectDirectory = baseDirectory;
      if (configStamp !== stamp) service.cleanupSemanticCache();
      configStamp = stamp;
      lastConfigRefresh = Date.now();
    };

    const refreshConfig = (requestedFile) => {
      let directory = path.dirname(requestedFile);
      let configFile;
      while (isInside(root, directory)) {
        configFile = [
          path.join(directory, "tsconfig.json"),
          path.join(directory, "jsconfig.json"),
        ].find(fileExists);
        if (configFile || directory === root) break;
        directory = path.dirname(directory);
      }
      const stamp = `${configFile ?? "inferred"}:${configFile ? ts.sys.getModifiedTime?.(configFile)?.getTime() : 0}`;
      if (stamp !== configStamp || Date.now() - lastConfigRefresh >= 2_000)
        parseConfig(configFile, stamp);
      inferredJavaScript = /\.[cm]?jsx?$/i.test(requestedFile) && !configuredOptions.allowJs;
      files = inferredJavaScript ? [] : configuredFiles;
      options = inferredJavaScript ? inferredOptions : configuredOptions;
    };

    const host = {
      getCompilationSettings: () => options,
      getScriptFileNames: () => [
        ...new Set([
          ...files,
          ...[...documents.keys()].filter((file) =>
            (inferredJavaScript ? /\.[cm]?jsx?$/i : /\.[cm]?[jt]sx?$/i).test(file),
          ),
        ]),
      ],
      getScriptVersion: (file) =>
        documents.has(path.resolve(file))
          ? `buffer:${documents.get(path.resolve(file)).revision}`
          : `disk:${allowed(file) ? ts.sys.getModifiedTime?.(file)?.getTime() : 0}`,
      getScriptSnapshot: (file) => {
        const text = readFile(file);
        return text === undefined ? undefined : ts.ScriptSnapshot.fromString(text);
      },
      getCurrentDirectory: () => projectDirectory,
      getDefaultLibFileName: (compilerOptions) =>
        path.join(libRoot, ts.getDefaultLibFileName(compilerOptions)),
      getProjectVersion: () => String(projectVersion),
      useCaseSensitiveFileNames: () => ts.sys.useCaseSensitiveFileNames,
      readFile,
      fileExists,
      readDirectory,
      directoryExists: (dir) => allowed(dir) && ts.sys.directoryExists(dir),
      getDirectories: (dir) =>
        allowed(dir)
          ? ts.sys.getDirectories(dir).filter((name) => allowed(path.join(dir, name)))
          : [],
      realpath: (file) =>
        allowed(file) ? (ts.sys.realpath && ts.sys.realpath(file)) || file : file,
    };
    const service = ts.createLanguageService(host);
    const preferences = {
      includeCompletionsForModuleExports: false,
      includeCompletionsWithInsertText: true,
      includeInlayParameterNameHints: "all",
      includeInlayParameterNameHintsWhenArgumentMatchesName: false,
      includeInlayFunctionParameterTypeHints: true,
      includeInlayVariableTypeHints: true,
      includeInlayPropertyDeclarationTypeHints: true,
      includeInlayFunctionLikeReturnTypeHints: true,
      includeInlayEnumMemberValueHints: true,
    };
    const location = (file, span) => {
      if (!isInside(root, file) || !allowed(file)) return undefined;
      const source = service.getProgram()?.getSourceFile(file);
      if (!source) return undefined;
      const start = source.getLineAndCharacterOfPosition(span.start);
      const end = source.getLineAndCharacterOfPosition(span.start + span.length);
      return {
        relativePath: path.relative(root, file).split(path.sep).join("/"),
        start: span.start,
        length: span.length,
        line: start.line,
        character: start.character,
        endLine: end.line,
        endCharacter: end.character,
      };
    };
    const fileEdits = (changes) => {
      if (changes.length > 200)
        throw new Error(
          "This refactoring affects more than 200 files. Narrow its scope before applying it.",
        );
      let total = 0;
      return changes.map((change) => {
        if (
          change.isNewFile ||
          !isInside(root, change.fileName) ||
          !allowed(change.fileName) ||
          path
            .relative(root, change.fileName)
            .split(path.sep)
            .some((part) => part === ".git" || part === "node_modules")
        ) {
          throw new Error(
            "This edit creates a file or changes a file outside the workspace; apply it manually.",
          );
        }
        const expectedContents = readFile(change.fileName);
        if (expectedContents === undefined)
          throw new Error("Cannot load the source for this edit.");
        total +=
          expectedContents.length +
          change.textChanges.reduce((sum, edit) => sum + edit.newText.length, 0);
        if (total > 8_000_000 || change.textChanges.length > 10_000)
          throw new Error(
            "This refactoring exceeds the safe edit transaction size. Narrow its scope before applying it.",
          );
        return {
          relativePath: path.relative(root, change.fileName).split(path.sep).join("/"),
          expectedContents,
          edits: change.textChanges.map(({ span, newText }) => ({
            start: span.start,
            length: span.length,
            newText,
          })),
        };
      });
    };
    const diagnostics = (file) =>
      [
        ...service.getSyntacticDiagnostics(file),
        ...service.getSemanticDiagnostics(file),
        ...service.getSuggestionDiagnostics(file),
      ].slice(0, MAX_RESULT_ITEMS);
    const diagnostic = (item) => ({
      start: item.start ?? 0,
      length: item.length ?? 0,
      severity:
        item.category === ts.DiagnosticCategory.Error
          ? "error"
          : item.category === ts.DiagnosticCategory.Warning
            ? "warning"
            : item.category === ts.DiagnosticCategory.Suggestion
              ? "hint"
              : "info",
      message: ts.flattenDiagnosticMessageText(item.messageText, "\n"),
      code: item.code,
      unnecessary: item.reportsUnnecessary === true,
      deprecated: item.reportsDeprecated === true,
    });

    return {
      request(input) {
        if (input.documents.reduce((sum, doc) => sum + doc.contents.length, 0) > 8_000_000)
          throw new Error("Open document snapshots exceed 8 MB.");
        documents = new Map(
          input.documents.map((doc) => {
            const absolute = resolve(doc.relativePath);
            const previous = documents.get(absolute);
            return [
              absolute,
              {
                contents: doc.contents,
                version: doc.version,
                revision: previous?.contents === doc.contents ? previous.revision : ++nextRevision,
              },
            ];
          }),
        );
        const file = resolve(input.relativePath);
        const document = documents.get(file);
        if (!document) throw new Error("A current document snapshot is required.");
        if (
          input.offset > document.contents.length ||
          input.offset + input.length > document.contents.length
        )
          throw new Error("Invalid document range.");
        projectVersion++;
        refreshConfig(file);
        const base = { version: document.version };
        const formatting = {
          indentSize: input.tabSize || 2,
          tabSize: input.tabSize || 2,
          convertTabsToSpaces: input.insertSpaces ?? true,
          newLineCharacter: "\n",
          semicolons: ts.SemicolonPreference.Ignore,
        };
        const span = { start: input.offset, length: input.length };
        switch (input.operation) {
          case "completion": {
            const result = service.getCompletionsAtPosition(file, input.offset, preferences);
            const word =
              /[$\p{ID_Continue}]*$/u.exec(document.contents.slice(0, input.offset))?.[0] ?? "";
            return {
              ...base,
              completions: (result?.entries ?? []).slice(0, MAX_RESULT_ITEMS).map((entry) => ({
                label: entry.name,
                kind: entry.kind,
                sortText: entry.sortText,
                insertText: entry.insertText ?? entry.name,
                replacement: entry.replacementSpan ??
                  result?.optionalReplacementSpan ?? {
                    start: input.offset - word.length,
                    length: word.length,
                  },
                ...(entry.source ? { source: entry.source } : {}),
              })),
            };
          }
          case "completionResolve": {
            if (!input.query) return base;
            const result = service.getCompletionEntryDetails(
              file,
              input.offset,
              input.query,
              formatting,
              input.completionSource,
              preferences,
              undefined,
            );
            return result
              ? {
                  ...base,
                  text: `${display(result.displayParts)}\n\n${display(result.documentation)}`,
                }
              : base;
          }
          case "hover": {
            const result = service.getQuickInfoAtPosition(file, input.offset);
            return result
              ? {
                  ...base,
                  text: `${display(result.displayParts)}\n\n${display(result.documentation)}`,
                  span: result.textSpan,
                }
              : base;
          }
          case "signature": {
            const result = service.getSignatureHelpItems(file, input.offset, undefined);
            return result
              ? {
                  ...base,
                  activeSignature: result.selectedItemIndex,
                  activeParameter: result.argumentIndex,
                  signatures: result.items.map((item) => ({
                    label:
                      display(item.prefixDisplayParts) +
                      item.parameters
                        .map((p) => display(p.displayParts))
                        .join(display(item.separatorDisplayParts)) +
                      display(item.suffixDisplayParts),
                    documentation: display(item.documentation),
                    parameters: item.parameters.map((p) => ({
                      label: display(p.displayParts),
                      documentation: display(p.documentation),
                    })),
                  })),
                }
              : base;
          }
          case "definition":
          case "typeDefinition":
          case "references": {
            const entries =
              input.operation === "typeDefinition"
                ? service.getTypeDefinitionAtPosition(file, input.offset)
                : input.operation === "references"
                  ? service.getReferencesAtPosition(file, input.offset)
                  : service.getDefinitionAtPosition(file, input.offset);
            return {
              ...base,
              locations: (entries ?? []).slice(0, MAX_RESULT_ITEMS).flatMap((entry) => {
                const value = location(entry.fileName, entry.textSpan);
                return value ? [value] : [];
              }),
            };
          }
          case "rename": {
            const info = service.getRenameInfo(file, input.offset, {
              allowRenameOfImportPath: false,
            });
            if (!info.canRename) throw new Error(info.localizedErrorMessage);
            if (!input.newName || !/^[$\p{ID_Start}][$\p{ID_Continue}]*$/u.test(input.newName))
              throw new Error("Enter a valid identifier.");
            const changes = new Map();
            for (const item of service.findRenameLocations(
              file,
              input.offset,
              false,
              false,
              true,
            ) ?? []) {
              const edits = changes.get(item.fileName) ?? [];
              edits.push({
                span: item.textSpan,
                newText: (item.prefixText ?? "") + input.newName + (item.suffixText ?? ""),
              });
              changes.set(item.fileName, edits);
            }
            const renamedFiles = fileEdits(
              [...changes].map(([fileName, textChanges]) => ({ fileName, textChanges })),
            );
            for (const change of renamedFiles) {
              const fileName = resolve(change.relativePath);
              let renamed = change.expectedContents;
              for (const edit of change.edits.slice().sort((a, b) => b.start - a.start)) {
                renamed =
                  renamed.slice(0, edit.start) +
                  edit.newText +
                  renamed.slice(edit.start + edit.length);
              }
              const validate = (contents) => {
                const source = ts.createSourceFile(
                  fileName,
                  contents,
                  options.target ?? ts.ScriptTarget.ESNext,
                  true,
                );
                const validationOptions = { ...options, noLib: true, noResolve: true };
                const program = ts.createProgram([fileName], validationOptions, {
                  ...ts.createCompilerHost(validationOptions),
                  getSourceFile: (requested) =>
                    path.resolve(requested) === fileName ? source : undefined,
                  fileExists: () => false,
                  readFile: () => undefined,
                  directoryExists: () => false,
                  getDirectories: () => [],
                });
                return [
                  ...program.getSyntacticDiagnostics(source),
                  ...program.getSemanticDiagnostics(source),
                ];
              };
              const existing = new Map();
              for (const diag of validate(change.expectedContents)) {
                existing.set(diag.code, (existing.get(diag.code) ?? 0) + 1);
              }
              for (const diag of validate(renamed)) {
                const remaining = existing.get(diag.code) ?? 0;
                if (!remaining) throw new Error("Enter a valid identifier for this location.");
                existing.set(diag.code, remaining - 1);
              }
            }
            return { ...base, span: info.triggerSpan, edits: renamedFiles };
          }
          case "documentSymbols": {
            const symbols = [];
            const visit = (item, container) => {
              const loc = location(file, item.nameSpan ?? item.spans[0]);
              const extent = location(file, item.spans[0]);
              if (loc)
                symbols.push({
                  name: item.text,
                  kind: item.kind,
                  containerName: container,
                  location: loc,
                  ...(extent ? { extent } : {}),
                });
              item.childItems?.forEach((child) => visit(child, item.text));
            };
            service.getNavigationTree(file).childItems?.forEach((child) => visit(child, ""));
            return { ...base, symbols: symbols.slice(0, MAX_RESULT_ITEMS) };
          }
          case "workspaceSymbols":
            return {
              ...base,
              symbols: service
                .getNavigateToItems(input.query ?? "", MAX_RESULT_ITEMS)
                .flatMap((item) => {
                  const loc = location(item.fileName, item.textSpan);
                  return loc
                    ? [
                        {
                          name: item.name,
                          kind: item.kind,
                          containerName: item.containerName ?? "",
                          location: loc,
                        },
                      ]
                    : [];
                }),
            };
          case "diagnostics":
            return { ...base, diagnostics: diagnostics(file).map(diagnostic) };
          case "codeActions": {
            const codes = diagnostics(file)
              .filter(
                (item) =>
                  (item.start ?? 0) <= span.start + span.length &&
                  (item.start ?? 0) + (item.length ?? 0) >= span.start,
              )
              .map((item) => item.code);
            const actions = [];
            for (const fix of service.getCodeFixesAtPosition(
              file,
              span.start,
              span.start + span.length,
              codes,
              formatting,
              preferences,
            )) {
              if (fix.commands?.length) continue;
              try {
                actions.push({
                  title: fix.description,
                  kind: "quickfix",
                  edits: fileEdits(fix.changes),
                });
              } catch {
                /* unsafe fixes are not advertised */
              }
            }
            for (const refactor of service.getApplicableRefactors(
              file,
              { pos: span.start, end: span.start + span.length },
              preferences,
            )) {
              for (const action of refactor.actions) {
                if (action.notApplicableReason) continue;
                try {
                  const result = service.getEditsForRefactor(
                    file,
                    formatting,
                    { pos: span.start, end: span.start + span.length },
                    refactor.name,
                    action.name,
                    preferences,
                  );
                  if (result && !result.commands?.length)
                    actions.push({
                      title: action.description,
                      kind: "refactor",
                      edits: fileEdits(result.edits),
                    });
                } catch {
                  /* only representable edits are offered */
                }
              }
            }
            return { ...base, actions };
          }
          case "format":
            return {
              ...base,
              edits: fileEdits([
                {
                  fileName: file,
                  textChanges: input.length
                    ? service.getFormattingEditsForRange(
                        file,
                        span.start,
                        span.start + span.length,
                        formatting,
                      )
                    : service.getFormattingEditsForDocument(file, formatting),
                },
              ]),
            };
          case "organizeImports":
            return {
              ...base,
              edits: fileEdits(
                service.organizeImports({ type: "file", fileName: file }, formatting, preferences),
              ),
            };
          case "inlayHints":
            return {
              ...base,
              hints: service
                .provideInlayHints(
                  file,
                  span.length ? span : { start: 0, length: document.contents.length },
                  preferences,
                )
                .slice(0, MAX_RESULT_ITEMS)
                .map((hint) => ({
                  offset: hint.position,
                  text: hint.text,
                  kind: hint.kind === ts.InlayHintKind.Parameter ? "parameter" : "type",
                  whitespaceBefore: hint.whitespaceBefore ?? false,
                  whitespaceAfter: hint.whitespaceAfter ?? false,
                })),
            };
          case "folding":
            return {
              ...base,
              folding: service
                .getOutliningSpans(file)
                .slice(0, MAX_RESULT_ITEMS)
                .map((item) => item.textSpan),
            };
          default:
            throw new Error(`unknown editor operation ${input.operation}`);
        }
      },
    };
  }

  const workspaceRoot = process.env.SYNARA_TS_WORKSPACE || process.argv[2];
  if (!workspaceRoot) {
    process.stdout.write(JSON.stringify({ id: "init", error: "workspace root required" }) + "\n");
    process.exit(2);
  }
  const service = createService(workspaceRoot);
  process.stdout.write(JSON.stringify({ id: "init", ok: true }) + "\n");

  const rl = readline.createInterface({ input: process.stdin, terminal: false });
  rl.on("line", (line) => {
    let msg;
    try {
      msg = JSON.parse(line);
    } catch {
      return;
    }
    const id = msg.id;
    try {
      const result = service.request(msg.input);
      process.stdout.write(JSON.stringify({ id, result }) + "\n");
    } catch (error) {
      process.stdout.write(
        JSON.stringify({ id, error: error instanceof Error ? error.message : String(error) }) +
          "\n",
      );
    }
  });
}
