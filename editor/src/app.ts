import * as monaco from 'monaco-editor';
import wabt_loader from 'wabt';

import { loadWASM } from 'onigasm' // peer dependency of 'monaco-textmate'
import { Registry } from 'monaco-textmate' // peer dependency
import { wireTmGrammars } from 'monaco-editor-textmate'
import * as onigasm_bytes from 'data-url:../node_modules/onigasm/lib/onigasm.wasm'
import * as wat_json from './wat.json'
import * as theme_json from './my-theme.json'

let problem_name = location.pathname.split("/")[2];

async function start() {
    await loadWASM(onigasm_bytes);

    const registry = new Registry({
        getGrammarDefinition: async (scopeName) => {
            console.log()
            return {
                format: 'json',
                content: wat_json
            }
        }
    })

    // map of monaco "language id's" to TextMate scopeNames
    const grammars = new Map()
    grammars.set('wat', 'source.wat')

    monaco.editor.defineTheme('rainbowdrops-color-dark', {
        ...theme_json,
        base: 'vs-dark'
    });

    // this will fill with the template if the stored value is empty haha
    let initial = localStorage.getItem(problem_name) || await (await fetch("template")).text();
    let editor = monaco.editor.create(document.body, {
        value: initial,
        language: 'wat',
        theme: 'rainbowdrops-color-dark'
    });

    monaco.languages.register({
        id: 'wat',
    });

    await wireTmGrammars(monaco, registry, grammars, editor);

    editor.onDidChangeModelContent(async e => {
        let content = editor.getValue();
        localStorage.setItem(problem_name, content);

        await update_errors(editor);
    })

    editor.addAction({
        id: 'submit',
        label: 'submit the code!',
        run: async function (editor: monaco.editor.ICodeEditor, ...args: any[]): Promise<void> {
            let text = editor.getValue();
            let wabt = await wabt_loader();
            try {
                let module = wabt.parseWat("input.wat", text);
                module.validate();
                let bin = module.toBinary({ write_debug_names: false, relocatable: false, canonicalize_lebs: false });

                const formData = new FormData();
                formData.append("wasm", new Blob([bin.buffer]));
                const response = await fetch("../" + problem_name, {
                    method: "POST",
                    body: formData
                });
                if (response.redirected) {
                    open(response.url);
                } else {
                    let msg = await response.text();
                    alert(msg);
                }
            } catch (error) {
                if (error instanceof Error) {
                    alert(error.message);
                } else {
                    console.error(error);
                }
            }
        }
    })
}

start();

async function update_errors(editor: monaco.editor.ICodeEditor) {
    monaco.editor.removeAllMarkers("wabt");

    let text = editor.getValue();
    let wabt = await wabt_loader();
    try {
        let module = wabt.parseWat("input.wat", text);
        module.validate();
    } catch (error) {
        if (error instanceof Error) {
            let msgs = error.message.split("\n").slice(1, -1);

            let markers: monaco.editor.IMarkerData[] = [];
            console.assert(msgs.length % 3 == 0);
            for (let i = 0; i < msgs.length; i += 3) {
                let [loc, msg] = msgs[i].split(" error: ");
                let line = parseInt(loc.split(":")[1]);
                let start = msgs[i + 2].indexOf("^");
                let end = msgs[i + 2].lastIndexOf("^") + 1;

                markers.push({
                    severity: monaco.MarkerSeverity.Error,
                    message: msg,
                    startLineNumber: line,
                    startColumn: start + 1,
                    endLineNumber: line,
                    endColumn: end + 1
                });
            }

            let model = editor.getModel();
            if (model) {
                monaco.editor.setModelMarkers(model, "wabt", markers);
            }
        } else {
            console.error(error);
        }
    }
}