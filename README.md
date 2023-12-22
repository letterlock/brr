# brr  

<p align="center">
  <img src="https://raw.githubusercontent.com/letterlock/brr/master/assets/brr.png" width="15%">
</p>

## the perfunctory prose proliferator  
> original programmery provided by the passionately proficient progenitors of [kilo](https://github.com/antirez/kilo) and [hecto](https://github.com/pflenker/hecto-tutorial).  
> amateurishly adapted and apprehensively altered by maxwell letterlock.  

brr is a text editor that adapts some of the permanency of writing by hand or on a typewriter. the intention with this is to hopefully curb the urge to constantly change what one has just written down instead of writing something new.  

for this reason, it consists of an intentionally minimal main set of "features":  
- brr cannot edit saved files except to add to them  
- once you start writing, brr will save every few words (six by default)  
- after a period of inactivity, (five seconds by default) brr will save the file upon the next keypress  

it was my first ever project and i never finished its original c-based incarnation. eventually, i decided i would do the easy and straightforward thing and port it to rust. :^)  
i have no clue what i'm doing, my code is very inefficient and probably downright stupid in some places, but everything seems to be working! if you do fix or tidy something, i would very much appreciate it if you would submit a pull and explain how and why you did what you did, so i can learn!  

finally, just to be clear: **you should back up any really important documents before editing them with brr.** i've done all i can to make sure nothing happens, and i'll be using brr myself, so i have an interest in making it not delete things i care about, but i'm a novice amateur programmer *at best*. for this reason -- as with any time you're trusting a file you care about to a stranger's program -- be careful!

## more features  
- editing *and* viewing mode -- you can scroll back through your file! (wow!)  
- a slightly inaccurate word count -- you can kind of see how much you've written when you're done! (incredible!)  
- soft word wrapping! don't look at the code for this! (unprecedented!)  
- ability to open a different file without leaving the program! (revolutionary!)  
- a config file! (extraordinary!)  
- editable text is highlighted! (inconceivable!)

## installation  
the easiest way to install brr is to just download the executable file, put it where you want, and run it. packaging and distributing brr through any kind of package system is a bit beyond my current expertise (and will to learn), so i'll leave it with simple binaries for now. if you download the linux binary, remember to give it permission to run.

#### virus problems  
i've had issues with the windows binary being flagged by antivirus software, which is apparently a common issue. i'm not sure what to do about that, the only thing i've heard that maybe helps is signing it, but i don't have the means for that. i've scanned it myself and with [virustotal](https://www.virustotal.com/gui/file/ab1f1775cae053f2bcef9fb43385cd51e398f8425b307df0d29819983c58864b?nocache=1).  
if you're nervous about it, that's probably good! it's smart not to trust some random .exe from the internet. you can peruse the source code for anything malicious if you know how, and if you don't know how but still trust my code, you can check out the below instructions to compile it from source. doing things that way will avoid you having to trust that i'm not injecting some malicious code before uploading it.  

#### compiling from source  
if you want to compile from source, the easiest way is to use the native rust package manager cargo. you can install cargo with [this guide](https://doc.rust-lang.org/stable/cargo/getting-started/installation.html).  
after you have cargo (and therefore the rust compiler), you can just run the following in a terminal:
```
$ cargo install brr-editor
```
to install.  
once installed, please note that brr will be added to your path as "brr-editor" (someone already had the crate name "brr" taken), so you can call it from anywhere. if you'd rather just call "brr", you can alias it, just be sure you don't have any other binary by that name in your path.

## usage  
brr can be called from the terminal or run from its executable.  
if you run it directly without arguments, you will reach a prompt asking you to type in the name of the file you want to open. if you run it with the name of the file you want to open like so:  
```
$ brr example.txt
```  
it will open that file for editing instead of showing you the prompt.  

regardless of how you start the application, the file name you provide can be a either a direct path or just the name of a file in your current operating folder.  
if you opened brr from the executable, your current operating folder should be set to the folder the executable is in.  
by default, if you type in a file name and forget to add ".txt" or ".md" on the end of it, brr will search for a file with one of those extensions in the folder you're working within and load that instead. this feature can be turned off in the config file.  
if you add brr to your path, you can of course call it from anywhere like a typical linux terminal text editor.  

once you're editing a file, you'll see the text you can actually affect appears highlighted, while the saved text appears normal. your cursor will be in the middle of the terminal window, and the text will scroll instead of the cursor, similar to a typewriter.  

if you want to take a break from writing and look over what you've written, you can press "ctrl+e" to change to view mode, or "ctrl+h" for helpful keybinds. "ctrl+s" saves and "ctrl+o" will allow you to open a new file in the same way as above.  

## configuration  
brr uses a simple plaintext config file that should be fairly straightforward to use, just open it in your favourite (actually functional) text editor and change the values after the equals symbols! the 'brr.conf.default' file contains all the default values and syntax, as well as some explanations for the various options.  
on opening, brr will check the directory containing its executable for a 'brr.conf' file. if you're on linux, brr will first check "$XDG_CONFIG_HOME/brr" (if this is unset, it will also just check ~/.config/brr), before checking its own directory.  

## logging and errors
if you're on windows, brr reports all errors through a log file placed in the same directory as the executable. if you're on linux, brr will first try to find (and create, if absent) "$XDG_STATE_HOME/brr". if $XDG_STATE_HOME is unset, it will default to "~/.local/state/brr". if it can't create that, it will just try to put the log in the directory containing its executable.  
if brr encounters an unrecoverable error, it will panic and die, leaving you with the error that killed it. if you opened the executable from outside of a terminal (e.g. you just ran the executable), the terminal will close as the process exits, so you'll have to check the log for what happened.  
if you do encounter an error that kills brr, please report it as an issue, include your log file, and tell me what you were doing when brr was die.  

## attributions
brr was originally adapted from [this](https://viewsourcecode.org/snaptoken/kilo/01.setup.html) tutorial, and later ported with the help of [this](https://www.flenker.blog/hecto/) one. i also took a lot of inspiration from the way [kibi](https://github.com/ilai-deutel/kibi) does things, it's a very cool project that is a very faithful implimentation of the original kilo project in rust. the code is extremely well documented and flexible, and it does some very clever things to achieve such a tiny codebase.  
