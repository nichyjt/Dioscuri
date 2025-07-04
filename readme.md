# Dioscuri

Yet another [Gemini](http://portal.mozz.us/gemini/geminiprotocol.net/) protocol implementation in Rust.  

Dioscuri is currently in **Beta**.
- this client supports basic surfing and browsing :white_check_mark:
- this client DOES NOT support user-state management yet (i.e. you provide your own cert) :x:
- this client DOES NOT support any customizability yet :x:

## Vision
Dioscuri aims to be a hackable, accessible way to access hobbyist network protocols such as Gemini.

**Hackable**
- Eventually, Dioscuri aims to allow you to roll your own html,css,js and themes to customize your experience.
  Similar vibes to old school Netscape or building your own Blogger website!

**Accessible**
- No need to install fancy GUI dependencies such as wxWidgets or curses or whatever which you won't use in a month's time
- Simply access Gemini from the convenience of a good-ol regular web browser!

## Architecture 

Each box denotes a submodule in the project.

![docs/](docs/arch.png)

We adhere strictly (within reason) to [single-responsibility](https://en.wikipedia.org/wiki/Single-responsibility_principle) principle.  
As such, each module should be doing semantically distinct things and not contaminate one another.  

## Installation
Unfortunately, you will have to build the binary for now.  

First, install cargo/rust on your machine.  
Then, fork this repo and run: `$ cargo run`.  
Open the client via any browser on `localhost:1965`.  
Enjoy Gemini!  