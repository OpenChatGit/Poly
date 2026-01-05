// Test file with intentional errors and warnings for Monaco diagnostics

// ERROR: Syntax error - missing closing parenthesis
function brokenFunction(a, b {
  return a + b;
}

// ERROR: Undefined variable
console.log(undefinedVariable);

// ERROR: Cannot read property of undefined
const obj = null;
obj.property = "test";

// WARNING: Unused variable
const unusedVar = 42;

// WARNING: Variable declared but never used
let neverUsed = "hello";

// ERROR: Duplicate declaration
let duplicateVar = 1;
let duplicateVar = 2;

// WARNING: Unreachable code
function unreachableCode() {
  return true;
  console.log("This will never run");
}

// ERROR: Invalid assignment
const constantValue = 10;
constantValue = 20;

// WARNING: Implicit any (would show in strict TS)
function noTypes(x, y) {
  return x + y;
}

// ERROR: Missing argument
function requiresArg(required) {
  return required * 2;
}
requiresArg();

// WARNING: Comparison always false
if (5 === "5") {
  console.log("Never happens");
}

// ERROR: Invalid regex
const badRegex = /[/;

// WARNING: Empty block
if (true) {
}

// ERROR: Unexpected token
const broken = {
  key: "value"
  anotherKey: "missing comma"
};

// WARNING: Deprecated API usage hint
document.write("Don't use this");

// ERROR: Type mismatch in operation
const num = 5;
const str = "hello";
const result = num - str;

console.log("End of test file");
