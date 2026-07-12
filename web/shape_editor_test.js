#!/usr/bin/env node
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const indexPath = path.join(__dirname, "index.html");
const html = fs.readFileSync(indexPath, "utf8");

function sourceBetween(startMarker, endMarker) {
  const start = html.indexOf(startMarker);
  assert.notEqual(start, -1, `missing ${startMarker}`);
  const end = html.indexOf(endMarker, start);
  assert.notEqual(end, -1, `missing ${endMarker}`);
  return html.slice(start, end);
}

const shapeEditorSource =
  sourceBetween("function loadShapeSample()", "window.addEventListener");

const elements = new Map([
  ["shape-name", { value: "" }],
  ["shape-radius", { value: "" }],
  ["shape-points", { value: "" }],
]);

const context = {
  window: { __dynalogoCommands: [] },
  document: {
    getElementById(id) {
      const element = elements.get(id);
      assert.ok(element, `unexpected element lookup: ${id}`);
      return element;
    },
  },
};
vm.createContext(context);
vm.runInContext(shapeEditorSource, context, { filename: "web/index.html#shape-editor" });

context.loadShapeSample();
assert.equal(elements.get("shape-name").value, "diamond");
assert.equal(elements.get("shape-radius").value, "12");
assert.equal(
  elements.get("shape-points").value,
  "[[0 12] [8 0] [0 -12] [-8 0]]",
);

context.window.__dynalogoCommands.length = 0;
context.queueShapeCommand(false);
assert.deepEqual(context.window.__dynalogoCommands, [
  'putsh "diamond [[0 12] [8 0] [0 -12] [-8 0]]',
]);

context.window.__dynalogoCommands.length = 0;
context.queueShapeCommand(true);
assert.deepEqual(context.window.__dynalogoCommands, [
  ['putsh "diamond [[0 12] [8 0] [0 -12] [-8 0]]', 'setshape "diamond 12'].join("\n"),
]);

elements.get("shape-name").value = "";
context.window.__dynalogoCommands.length = 0;
context.queueShapeCommand(true);
assert.deepEqual(context.window.__dynalogoCommands, []);

console.log("shape editor web tests passed");
