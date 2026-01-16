# Poly API Reference

Complete reference for all built-in functions in the Poly programming language.

## Table of Contents

- [Core Functions](#core-functions)
- [Type Conversion](#type-conversion)
- [String Methods](#string-methods)
- [List Methods](#list-methods)
- [Math Functions](#math-functions)
- [Random Functions](#random-functions)
- [File I/O](#file-io)
- [Path Utilities](#path-utilities)
- [HTTP Functions](#http-functions)
- [HTTP Streaming](#http-streaming)
- [HTTP Server](#http-server)
- [JSON Functions](#json-functions)
- [System Functions](#system-functions)
- [Hashing & Encoding](#hashing--encoding)
- [Regex Functions](#regex-functions)
- [Parallel Processing](#parallel-processing)
- [HTML Generation](#html-generation)
- [Web Framework](#web-framework)

---

## Core Functions

### print(*args)
Prints values to stdout.
```poly
print("Hello", "World")  # Output: Hello World
print(42, true, [1, 2])  # Output: 42 true [1, 2]
```

### len(obj)
Returns the length of a string, list, or dict.
```poly
len("hello")      # 5
len([1, 2, 3])    # 3
len({"a": 1})     # 1
```

### range(end) / range(start, end) / range(start, end, step)
Creates a list of integers.
```poly
range(5)          # [0, 1, 2, 3, 4]
range(2, 5)       # [2, 3, 4]
range(0, 10, 2)   # [0, 2, 4, 6, 8]
range(5, 0, -1)   # [5, 4, 3, 2, 1]
```

### abs(n)
Returns absolute value.
```poly
abs(-5)    # 5
abs(3.14)  # 3.14
```

### min(*args) / min(list)
Returns minimum value.
```poly
min(3, 1, 4)      # 1
min([3, 1, 4])    # 1
```

### max(*args) / max(list)
Returns maximum value.
```poly
max(3, 1, 4)      # 4
max([3, 1, 4])    # 4
```

### sum(list)
Returns sum of all integers in a list.
```poly
sum([1, 2, 3, 4])  # 10
```

### sorted(list)
Returns a sorted copy of the list.
```poly
sorted([3, 1, 4, 1, 5])  # [1, 1, 3, 4, 5]
```

### reversed(list) / reversed(string)
Returns reversed list.
```poly
reversed([1, 2, 3])  # [3, 2, 1]
reversed("hello")    # ["o", "l", "l", "e", "h"]
```

### enumerate(list)
Returns list of [index, value] pairs.
```poly
enumerate(["a", "b", "c"])  # [[0, "a"], [1, "b"], [2, "c"]]
```

### zip(list1, list2)
Combines two lists into pairs.
```poly
zip([1, 2], ["a", "b"])  # [[1, "a"], [2, "b"]]
```

### any(list)
Returns true if any element is truthy.
```poly
any([false, true, false])  # true
any([0, 0, 0])             # false
```

### all(list)
Returns true if all elements are truthy.
```poly
all([true, true, true])  # true
all([true, false])       # false
```

---

## Type Conversion

### str(value)
Converts value to string.
```poly
str(42)      # "42"
str(true)    # "true"
str([1, 2])  # "[1, 2]"
```

### int(value)
Converts to integer.
```poly
int("42")    # 42
int(3.14)    # 3
int(true)    # 1
```

### float(value)
Converts to float.
```poly
float("3.14")  # 3.14
float(42)      # 42.0
```

### bool(value)
Converts to boolean.
```poly
bool(1)       # true
bool(0)       # false
bool("")      # false
bool("hi")    # true
```

### list(iterable)
Converts to list.
```poly
list("abc")  # ["a", "b", "c"]
```

### type(value)
Returns type name as string.
```poly
type(42)       # "int"
type("hi")     # "str"
type([1, 2])   # "list"
type({"a": 1}) # "dict"
```

### chr(n)
Returns character from Unicode code point.
```poly
chr(65)   # "A"
chr(8364) # "€"
```

### ord(char)
Returns Unicode code point of character.
```poly
ord("A")  # 65
ord("€")  # 8364
```

### hex(n)
Converts integer to hex string.
```poly
hex(255)  # "0xff"
```

### bin(n)
Converts integer to binary string.
```poly
bin(10)  # "0b1010"
```

### oct(n)
Converts integer to octal string.
```poly
oct(8)  # "0o10"
```

### round(n, digits?)
Rounds a number.
```poly
round(3.14159)     # 3
round(3.14159, 2)  # 3.14
```

---

## String Methods

### upper(s)
Converts string to uppercase.
```poly
upper("hello")  # "HELLO"
```

### lower(s)
Converts string to lowercase.
```poly
lower("HELLO")  # "hello"
```

### strip(s)
Removes leading/trailing whitespace.
```poly
strip("  hello  ")  # "hello"
```

### split(s, sep?)
Splits string by separator (whitespace if not specified).
```poly
split("a,b,c", ",")  # ["a", "b", "c"]
split("hello world") # ["hello", "world"]
```

### join(sep, list)
Joins list elements with separator.
```poly
join(", ", ["a", "b", "c"])  # "a, b, c"
```

### replace(s, old, new)
Replaces all occurrences.
```poly
replace("hello", "l", "L")  # "heLLo"
```

### startswith(s, prefix)
Checks if string starts with prefix.
```poly
startswith("hello", "he")  # true
```

### endswith(s, suffix)
Checks if string ends with suffix.
```poly
endswith("hello", "lo")  # true
```

### find(s, sub)
Returns index of substring (-1 if not found).
```poly
find("hello", "ll")  # 2
find("hello", "x")   # -1
```

### count(s, sub) / count(list, value)
Counts occurrences.
```poly
count("hello", "l")     # 2
count([1, 2, 1], 1)     # 2
```

### isdigit(s)
Checks if string contains only digits.
```poly
isdigit("123")   # true
isdigit("12.3")  # false
```

### isalpha(s)
Checks if string contains only letters.
```poly
isalpha("hello")  # true
isalpha("hello1") # false
```

### isalnum(s)
Checks if string contains only letters and digits.
```poly
isalnum("hello123")  # true
isalnum("hello!")    # false
```

---

## List Methods

### push(list, value) / append(list, value)
Returns new list with value appended.
```poly
push([1, 2], 3)  # [1, 2, 3]
```

### pop(list)
Returns and removes last element.
```poly
pop([1, 2, 3])  # 3
```

### insert(list, index, value)
Returns new list with value inserted at index.
```poly
insert([1, 3], 1, 2)  # [1, 2, 3]
```

### remove(list, value)
Returns new list with first occurrence removed.
```poly
remove([1, 2, 3, 2], 2)  # [1, 3, 2]
```

### index(list, value)
Returns index of value (-1 if not found).
```poly
index([1, 2, 3], 2)  # 1
index([1, 2, 3], 5)  # -1
```

### clear(list) / clear(dict)
Returns empty list or dict.
```poly
clear([1, 2, 3])  # []
```

### copy(list) / copy(dict)
Returns shallow copy.
```poly
copy([1, 2, 3])  # [1, 2, 3]
```

### extend(list1, list2)
Returns new list with list2 appended.
```poly
extend([1, 2], [3, 4])  # [1, 2, 3, 4]
```

---

## Math Functions

### math_sqrt(n)
Square root.
```poly
math_sqrt(16)  # 4.0
```

### math_sin(n)
Sine (radians).
```poly
math_sin(0)  # 0.0
```

### math_cos(n)
Cosine (radians).
```poly
math_cos(0)  # 1.0
```

### math_tan(n)
Tangent (radians).
```poly
math_tan(0)  # 0.0
```

### math_floor(n)
Floor (round down).
```poly
math_floor(3.7)  # 3
```

### math_ceil(n)
Ceiling (round up).
```poly
math_ceil(3.2)  # 4
```

### math_log(n)
Natural logarithm.
```poly
math_log(2.718281828)  # ~1.0
```

---

## Random Functions

### random()
Returns random float between 0 and 1.
```poly
random()  # 0.7234...
```

### randint(a, b)
Returns random integer between a and b (inclusive).
```poly
randint(1, 10)  # 7
```

### choice(list)
Returns random element from list.
```poly
choice(["a", "b", "c"])  # "b"
```

---

## File I/O

### read_file(path)
Reads file content as string.
```poly
let content = read_file("config.json")
```

### read_file_base64(path)
Reads file as base64 data URL.
```poly
let img = read_file_base64("image.png")
# "data:image/png;base64,iVBORw0..."
```

### write_file(path, content)
Writes string to file.
```poly
write_file("output.txt", "Hello World")
```

### file_exists(path)
Checks if file exists.
```poly
file_exists("config.json")  # true/false
```

### list_dir(path?)
Lists directory contents.
```poly
list_dir(".")           # ["file1.txt", "folder", ...]
list_dir("/home/user")  # [...]
```

### mkdir(path)
Creates directory (including parents).
```poly
mkdir("path/to/new/dir")  # true/false
```

### remove_file(path)
Deletes a file.
```poly
remove_file("temp.txt")  # true/false
```

---

## Path Utilities

### path_join(parts...)
Joins path components.
```poly
path_join("home", "user", "file.txt")  # "home/user/file.txt"
```

### path_exists(path)
Checks if path exists.
```poly
path_exists("/home/user")  # true/false
```

### path_basename(path)
Returns filename from path.
```poly
path_basename("/home/user/file.txt")  # "file.txt"
```

### path_dirname(path)
Returns directory from path.
```poly
path_dirname("/home/user/file.txt")  # "/home/user"
```

### path_ext(path)
Returns file extension.
```poly
path_ext("file.txt")  # "txt"
```

---

## HTTP Functions

### http_get(url)
Makes HTTP GET request.
```poly
let resp = http_get("https://api.example.com/data")
# Returns: {"status": 200, "body": "..."}
```

### http_get_steam(url)
Makes HTTP GET request with Steam session cookies.
```poly
let resp = http_get_steam("https://store.steampowered.com/api/...")
# Includes steamLoginSecure cookie for authenticated requests
```

### http_get_parallel(urls)
Makes multiple HTTP GET requests in parallel.
```poly
let urls = ["https://api.com/1", "https://api.com/2", "https://api.com/3"]
let responses = http_get_parallel(urls)
# Returns list of {"status": 200, "body": "...", "url": "..."} dicts
```

### http_post(url, body, content_type?)
Makes HTTP POST request.
```poly
let resp = http_post("https://api.com/data", '{"key": "value"}', "application/json")
# Returns: {"status": 200, "body": "..."}
```

### http_post_json(url, data)
Makes HTTP POST request with JSON body.
```poly
let resp = http_post_json("https://api.com/data", {"key": "value"})
# Automatically serializes dict to JSON
# Returns: {"status": 200, "body": {...}}  (body is parsed JSON)
```

---

## HTTP Streaming

For streaming responses (SSE, NDJSON).

### http_stream_start(url, body)
Starts a streaming POST request in background.
```poly
let session_id = http_stream_start("https://api.com/stream", {"prompt": "Hello"})
```

### http_stream_poll(session_id)
Polls for new chunks from stream.
```poly
let result = http_stream_poll(session_id)
# Returns: {"chunks": ["line1", "line2"], "done": false, "error": null}
```

### http_stream_close(session_id)
Closes streaming session.
```poly
http_stream_close(session_id)  # true/false
```

---

## HTTP Server

For OAuth callbacks, webhooks, etc.

### http_server_start(port, response_html?)
Starts a local HTTP server.
```poly
http_server_start(8080, "<h1>Success!</h1>")
```

### http_server_poll()
Gets pending requests.
```poly
let requests = http_server_poll()
# Returns list of request dicts or None
# Each request: {"method": "GET", "path": "/callback", "query": "code=abc", 
#                "query_params": {"code": "abc"}, "headers": {...}, "body": ""}
```

### http_server_stop()
Stops the HTTP server.
```poly
http_server_stop()  # true
```

---

## JSON Functions

### json_parse(string)
Parses JSON string to Poly value.
```poly
let data = json_parse('{"name": "John", "age": 30}')
print(data["name"])  # John
```

### json_stringify(value)
Converts Poly value to JSON string.
```poly
let json = json_stringify({"name": "John", "items": [1, 2, 3]})
# '{"name":"John","items":[1,2,3]}'
```

---

## System Functions

### env()
Returns all environment variables as dict.
```poly
let vars = env()
print(vars["PATH"])
```

### env_get(name, default?)
Gets environment variable.
```poly
let home = env_get("HOME", "/tmp")
```

### env_set(name, value)
Sets environment variable.
```poly
env_set("MY_VAR", "value")
```

### exec(command)
Executes shell command.
```poly
let result = exec("ls -la")
# Returns: {"stdout": "...", "stderr": "...", "code": 0}
```

### timestamp()
Returns milliseconds since Unix epoch.
```poly
timestamp()  # 1705420800000
```

### datetime()
Returns current date/time as dict.
```poly
datetime()
# {"year": 2024, "month": 1, "day": 16, "hour": 12, "minute": 30, "second": 45}
```

### sleep_ms(ms)
Sleeps for milliseconds.
```poly
sleep_ms(1000)  # Sleep 1 second
```

### uuid()
Generates UUID v4.
```poly
uuid()  # "550e8400-e29b-41d4-a716-446655440000"
```

### time()
Returns seconds since Unix epoch as float.
```poly
time()  # 1705420800.123
```

### sleep(seconds)
Sleeps for seconds.
```poly
sleep(1.5)  # Sleep 1.5 seconds
```

---

## Hashing & Encoding

### hash_md5(string)
Returns MD5 hash (hex).
```poly
hash_md5("hello")  # "5d41402abc4b2a76b9719d911017c592"
```

### hash_sha256(string)
Returns SHA256 hash (hex).
```poly
hash_sha256("hello")  # "2cf24dba5fb0a30e..."
```

### base64_encode(string)
Encodes string to base64.
```poly
base64_encode("Hello")  # "SGVsbG8="
```

### base64_decode(string)
Decodes base64 string.
```poly
base64_decode("SGVsbG8=")  # "Hello"
```

---

## Regex Functions

### regex_match(pattern, string)
Tests if pattern matches string.
```poly
regex_match(r"\d+", "abc123")  # true
regex_match(r"^\d+$", "abc")   # false
```

### regex_find(pattern, string)
Finds all matches.
```poly
regex_find(r"\d+", "a1b22c333")  # ["1", "22", "333"]
```

### regex_replace(pattern, replacement, string)
Replaces all matches.
```poly
regex_replace(r"\d+", "X", "a1b2c3")  # "aXbXcX"
```

---

## Parallel Processing

### parallel_map(fn, list)
Applies function to each element (parallel execution).
```poly
fn double(x):
    return x * 2

parallel_map(double, [1, 2, 3, 4])  # [2, 4, 6, 8]
```

### parallel_filter(fn, list)
Filters list using function (parallel execution).
```poly
fn is_even(x):
    return x % 2 == 0

parallel_filter(is_even, [1, 2, 3, 4, 5, 6])  # [2, 4, 6]
```

---

## HTML Generation

### html(title, body, styles?, scripts?)
Creates complete HTML document.
```poly
let page = html("My App", "<h1>Hello</h1>", "body { color: red; }", "console.log('hi')")
write_file("web/index.html", page)
```

### html_escape(string)
Escapes HTML special characters.
```poly
html_escape("<script>alert('xss')</script>")
# "&lt;script&gt;alert('xss')&lt;/script&gt;"
```

### html_tag(tag, content, attrs?)
Creates HTML tag.
```poly
html_tag("div", "Hello", {"class": "container", "id": "main"})
# '<div class="container" id="main">Hello</div>'

html_tag("img", "", {"src": "photo.jpg"})
# '<img src="photo.jpg" />'
```

---

## Web Framework

### router(routes, not_found?)
Creates client-side router.
```poly
let routes = {
    "/": "<h1>Home</h1>",
    "/about": "<h1>About</h1>"
}
let router_js = router(routes, "<h1>404</h1>")
```

Generated JavaScript provides:
- Hash-based routing (`#/path`)
- `navigate(path)` function
- `routechange` event

### route(path, html)
Creates single route entry (helper for router).
```poly
let r = route("/home", "<h1>Home</h1>")
```

### component(name, template, props?)
Creates reusable component function.
```poly
let card_js = component("Card", """
<div class="card">
    <h2>${title}</h2>
    <p>${content}</p>
</div>
""", ["title", "content"])
```

Usage in JavaScript:
```javascript
Card("Hello", "World")  // Returns HTML string
```

### store(name, initial, actions?)
Creates reactive state store.
```poly
let initial = {"count": 0}
let actions = {
    "increment": "this.state.count++",
    "decrement": "this.state.count--"
}
let store_js = store("Counter", initial, actions)
```

Generated JavaScript provides:
```javascript
counterStore.getState()      // Get current state
counterStore.increment()     // Call action
counterStore.subscribe(fn)   // Subscribe to changes
```

### live_reload(port?)
Generates WebSocket live reload script.
```poly
let reload_js = live_reload(3001)
```

---

## Operators

### Arithmetic
- `+` Addition / String concatenation / List concatenation
- `-` Subtraction
- `*` Multiplication / String repeat
- `/` Division (returns float)
- `//` Floor division
- `%` Modulo
- `**` Power

### Comparison
- `==` Equal
- `!=` Not equal
- `<` Less than
- `>` Greater than
- `<=` Less than or equal
- `>=` Greater than or equal

### Logical
- `and` Logical AND
- `or` Logical OR
- `not` Logical NOT

### Membership
- `in` Check if item in list/string/dict
- `is` Identity comparison

---

## Running Poly

```bash
# Development server (hot reload)
poly dev .

# Run native app
poly run . --native

# Build project
poly build .
```
