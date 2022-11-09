# Autovor

Unofficial CLI for Endeavor

**This project is not associated with Endeavor**

## Usage

First of all create a file named `creds.txt` on the same folder you are going to run the program from. This file must contain **ONLY** 2 lines, the first being your login and the second being your password.

If you encounter any problems try running it with the `--emulate-browser` flag. This will make it so some _unnecessary_ GET requests are made in between operations, just like a browser would. If you find that this is necessary or something else doesn't work, please open an [issue](https://github.com/thomasqueirozb/autovor/issues/new).

There are also other flags. You can check them out by running `--help`.

## TODOs

- [ ] Make some sort of config file, so the amount of hours isn't hard coded (currently 8)
- [ ] Improve credential storage (somehow)
- [ ] Add flags to make this be able to run non-interactively
- [ ] Add a session/cookies caching mechanism, so logging in everytime wouldn't be required
- [ ] Internationalization support
- [ ] Publish as a crate (maybe)

## How it works

There is no kind of API or documentation so this project relies on the same endpoints used by the browser.
The content is returned as HTML so there is also some webscraping.
The HTTP return codes of the website don't actually indicate any errors (200 is returned most of the time) so error detection isn't (and probably never will be) perfect.

## Why does this exist?

The web interface is horrible and slow. It's a mobile interface reporpused to work in the browser. If UI was the only problem, I probably wouldn't have made this, but it's almost unusable. Every time you want to punch time you have to manually put in how many hours you worked and there is **no way** to punch multiple days at once, requiring you to go back, select another day and repeat the process.

I hate doing stuff like this so I made something to help me.

## Why Rust?

I intend to distribute this with my coworkers, some of which aren't developers. I had a fully functional version of this written in Python, but didn't want to bundle the Python interpreter and libraries when distributing this, nor did I want to have them install Python, `pip install` my package and also probably deal with PATH issues. Since Rust produces a single binary with no external dependencies (mostly), I chose to Rewrite it in Rust™.
