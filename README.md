=Streaming ChatGPT for Shell and Neovim=
AI assistants are transformational for programmers. These utilities attempts to bring these tools closer to the command-line and editor. There are three parts here:
1/ A Rust binary that streams completion responses to stdin
2/ A shell script that builds a little REPL over that binary
3/ A Neovim Lua plug-in that brings this functionality into the editor


==Rust program==
The Rust program can be built with `cargo build`. It expects an `OPENAI_API_KEY` environment variable. The Rust program can take two kinds of input, read from stdin:
1/ Raw input
In this case, a System prompt is provided in the compiled code
2/ Transcript
The Rust program also accepts a homegrown "transcript" format in which transcript sections are delineated by lines which look like this
```
===USER===
```
If a transcript does not start with a System section, then the default System prompt is used.

==Lua script==
The included lua script can be copied to `.config/nvim/lua` and installed with something like 
```
vim.cmd("command! ChatGPT lua require'chatgpt'.chatgpt()")
```
This plugin is optimized to allow for streaming. It attempts to keep new input in view by repositioning the cursor at the end of the buffer as new text is appended. The plugin takes care to work in the case that the user switches away from the window where the response is coming in. To turn off the cursor movement while a response is streaming, hit "Enter" or "Space." This will free the cursor the rest of the response.

==Shell script==
`shellbot.sh` can be used from the command line in cases where the editor isn't active. Because it uses `fold` for word wrap, it works best in a narrow window. The first prompt comes from $EDITOR. Subsequent prompts are taken with `read`. Hitting enter on a blank line does submit.
