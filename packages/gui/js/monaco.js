import * as monaco from 'monaco-editor';

window.initMonaco = function(code) {
    const editorElement = document.getElementById("editor");
    if (!editorElement) {
        console.error("Editor element not found");
        return;
    }

    const myEditor = monaco.editor.create(editorElement, {
        value: code,
        language: "yaml",
        automaticLayout: true,
    });

    window.monacoEditor = myEditor;
};