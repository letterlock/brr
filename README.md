# brr  

## the perfunctory prose proliferator  
> original programmery provided by the passionately proficient progenitors of [kilo](https://github.com/antirez/kilo) and [hecto](https://github.com/pflenker/hecto-tutorial).  
> amateurishly adapted and apprehensively altered by maxwell letterlock.  

brr is a text editor that adapts some of the permanency of writing by hand or on a typewriter in order to allow the user to produce text with as few edits as necessary, hopefully curbing the urge to constantly edit whatever they have written down.  

for this reason, it consists of an (intentionally) minimal main set of "features":  
- brr cannot edit saved files except to add to them  
- once you start writing, brr will save every few words (six by default)  
- after a period of inactivity, (five seconds by default) brr will save the file upon the next keypress  

it was my first ever project and i never finished its original c-based incarnation. eventually, i decided i would do the easy and straightforward thing and port it to rust. :^)  

brr was originally adapted from [this](https://viewsourcecode.org/snaptoken/kilo/01.setup.html) tutorial, and later ported with the help of [this](https://www.flenker.blog/hecto/) one. i also took a lot of inspiration from the way [kibi](https://github.com/ilai-deutel/kibi) does things, it's a very cool project that is a very faithful implimentation of the original kilo project in rust. the code is extremely well documented and flexible, and it does some very clever things to achieve such a tiny codebase.  

i still have no clue what i'm doing, my code is very inefficient and probably downright stupid in some places, but everything seems to be working! if you do find a bug, fix something or tidy something up, feel free to submit an issue or a pull request. if you do fix or tidy something, i would very much appreciate it if you would explain how and why, so i can learn!  

finally, just to be clear: **you should back up any really important documents before editing them with brr.** i've done all i can to make sure nothing happens, and i'll be using brr myself, so i have an interest in making it not delete things i care about, but i'm a novice amateur programmer *at best*. as with any time you're trusting a file you care about to a stranger's program -- be careful!

### more features  
- editing *and* viewing mode -- you can scroll back through your file! (wow!)  
- a slightly inaccurate word count -- you can kind of see how much you've written when you're done! (incredible!)  
- soft word wrapping! don't look at the code for this! (unprecedented!)  
- ability to open a different file without leaving the program! (revolutionary!)  
- a config file! (extraordinary!)  
- editable text is highlighted! (inconceivable!)

### installation  
todo  

#### compiling from source  
todo

### usage  
todo

### configuration  
brr uses a simple plaintext config file that should be fairly straightforward to use, just open it in your favourite (actually functional) text editor and change the values after the equals symbols! the 'brr.conf.default' file contains all the default values and syntax, as well as some explanations for the various options.
