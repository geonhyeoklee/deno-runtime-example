declare namespace runjs {
  function readFile(path: string): Promise<string>;
  function writeFile(path: string, content: string): Promise<void>;
  function removeFile(path: string): Promise<void>;
  function fetch(url: string): Promise<string>;
}

const path = "./log.txt";

try {
  const contents = await runjs.readFile(path);
  console.log("Read from a file", contents);
} catch (err) {
  console.error("Unable to read file", path, err);
}

await runjs.writeFile(path, "I can write to a file.");

const contents = await runjs.readFile(path);

console.log("Read from a file", path, "contents:", contents);
console.log("Removing file", path);

runjs.removeFile(path);
console.log("File removed");

interface Foo {
  bar: string;
  fizz: number;
}

const content = await runjs.fetch(
  "https://deno.land/std@0.177.0/examples/welcome.ts",
);
console.log("Content from fetch", content);