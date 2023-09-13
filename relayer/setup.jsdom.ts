
import { JSDOM } from "jsdom";
console.log("JSDOM setup start");

const dom = new JSDOM("<!doctype html><html><body></body></html>", {
  url: "http://localhost",
});

(global as any).window = dom.window;
(global as any).document = dom.window.document;
(global as any).navigator = dom.window.navigator;
console.log("JSDOM setup complete");
