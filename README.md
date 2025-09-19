# Intro
My attempt to solve the Synacor Challenge

Synacor Challenge is a hacking/programming challenge where you code an interpreter for a VM, then execute a program leading to more challenges

The VM specs are [here](./arch-spec)

The program is [here](./challenge.bin)

The official site is down (https://challenge.synacor.com/)

But you can validate codes via https://github.com/Aneurysm9/vm_challenge

# The emulator

To solve all puzzles, I implemented:
- basic emulation, I/O
- turn the emulator into a lib, controlled via CLI
- snapshots
- graph traversal and graphviz (dot) rendering
- tracing, breakpoints, assembler, disassembler
- function patching


# 1/8 arch-spec

The first code is in the specs

```
- Here's a code for the challenge website: duZSrqXIPoZA
```


# 2/8 Simple instructions

Implementing the first instructions `Noop`, `Out` and `Halt` will display the welcome message

```
Welcome to the Synacor Challenge!
Please record your progress by putting codes like
this one into the challenge website: hKRuXKPwTwlo
```

# 3/8 Self test

Implement the remaining instruction to complete the self-test

```
Executing self-test...

self-test complete, all tests pass
The self-test completion code is: NhMSeBfjuEsD
```

# 4/8 Tablet

We are now in some kind of text-based game

In Foothills (first room)

```
Things of interest here:
- tablet
```

```
$ i take tablet

Taken.
```

```
$ i use tablet


You find yourself writing "jtQUShZPqyoL" on the tablet.  Perhaps it's some kind of code?
```

# 5/8 Maze

Trying to explore the world is difficult, as there is a lot of rooms.

A graph traversal algorithm combined with Graphviz can create a nice map. Also the state loader/saver is useful to save time.

- Green: room with items
- Red: VM state == Halted

![maze](maze.svg)

Grab empty lantern and can

```
$ i use can

You fill your lantern with oil.  It seems to cheer up!
```

Find a message in a room

```
Chiseled on the wall of one of the passageways, you see:

    VdfKGsbnhQYK
```

Go to the dark passage (2 times west)

# 6/8 Ruins

We reuse the graph explorer to get a new map for the Ruins

![ruins](ruins.svg)

Problem statement

```
You stand in the massive central hall of these ruins.  The walls are crumbling, and vegetation has clearly taken over.  Rooms are attached in all directions.  There is a strange monument in the center of the hall with circular slots and unusual symbols.  It reads:

_ + _ * _^2 + _^3 - _ = 399
```

Grab the coins in different rooms

```
This coin is made of a blue metal.  It has nine dots on one side.
This coin is made of a red metal.  It has two dots on one side.
This coin is somehow still quite shiny.  It has a pentagon on one side.
This coin is slightly rounded, almost like a tiny bowl.  It has seven dots on one side.
This coin is somewhat corroded.  It has a triangle on one side.
```

Permutations solver:

```
use itertools::Itertools;

fn main() {
    let list = [2i32, 3, 5, 7, 9];
    
    for x in list.iter().permutations(5) {
        if x[0] + x[1] * x[2].pow(2) + x[3].pow(3) - x[4] == 399 {
            dbg!(x);
        }
    }
}
```

Solution:

```
[src/main.rs:8:13] x = [
    9,
    2,
    5,
    7,
    3,
]
```

```
You place the corroded coin into the leftmost open slot.
As you place the last coin, you hear a click from the north door.

$ i north

== Ruins ==
Because it has been so well-protected, this room hardly shows signs of decay.  The walls are covered in elaborate murals and decorated with precious metals and stones.

Things of interest here:
- teleporter

$ i use teleporter


You activate the teleporter!  As you spiral through time and space, you think you see a pattern in the stars...

    yjdhyEgpXPgW

After a few moments, you find yourself back on solid ground and a little disoriented.
```

# 7/8 Synacor Headquarters

This is the hardest code

The `strange book` gives some guidelines

```
The cover of this book subtly swirls with colors.  It is titled "A Brief Introduction to Interdimensional Physics".  It reads:

Recent advances in interdimensional physics have produced fascinating
predictions about the fundamentals of our universe!  For example,
interdimensional physics seems to predict that the universe is, at its root, a
purely mathematical construct, and that all events are caused by the
interactions between eight pockets of energy called "registers".
Furthermore, it seems that while the lower registers primarily control mundane
things like sound and light, the highest register (the so-called "eighth
register") is used to control interdimensional events such as teleportation.

A hypothetical such teleportation device would need to have have exactly two
destinations.  One destination would be used when the eighth register is at its
minimum energy level - this would be the default operation assuming the user
has no way to control the eighth register.  In this situation, the teleporter
should send the user to a preconfigured safe location as a default.

The second destination, however, is predicted to require a very specific
energy level in the eighth register.  The teleporter must take great care to
confirm that this energy level is exactly correct before teleporting its user!
If it is even slightly off, the user would (probably) arrive at the correct
location, but would briefly experience anomalies in the fabric of reality
itself - this is, of course, not recommended.  Any teleporter would need to test
the energy level in the eighth register and abort teleportation if it is not
exactly correct.

This required precision implies that the confirmation mechanism would be very
computationally expensive.  While this would likely not be an issue for large-
scale teleporters, a hypothetical hand-held teleporter would take billions of
years to compute the result and confirm that the eighth register is correct.

If you find yourself trapped in an alternate dimension with nothing but a
hand-held teleporter, you will need to extract the confirmation algorithm,
reimplement it on more powerful hardware, and optimize it.  This should, at the
very least, allow you to determine the value of the eighth register which would
have been accepted by the teleporter's confirmation mechanism.

Then, set the eighth register to this value, activate the teleporter, and
bypass the confirmation mechanism.  If the eighth register is set correctly, no
anomalies should be experienced, but beware - if it is set incorrectly, the
now-bypassed confirmation mechanism will not protect you!

Of course, since teleportation is impossible, this is all totally ridiculous.
```


reg7 = 1
instructions limit = 1.000.000.000

```
$ i use teleporter


A strange, electronic voice is projected into your mind:

  "Unusual setting detected!  Starting confirmation process!  Estimated time to completion: 1 billion years."
```

(emulator hangs for 30s, takes 22 GiB of RAM for stack allocations...)

Instrumenting the VM to count specific instructions shows that some functions are called a LOT. If we limit the number of instructions to run, we note that the following calls grow with this limit

Functions call graph to show the hot function: 6027

![teleporter](teleporter.svg)

Reimplement fn 6027 in rust
Increase stack size to 32 MiB due to high levels of recursion

Now setting fn_patching = true
After < 0.01s

```
A strange, electronic voice is projected into your mind:

  "Miscalibration detected!  Aborting teleportation!"

Nothing else seems to happen.
```

Now bute force register values


We find the solution : 25734

```
$ vm register set 7 25734
$ vm fn_patching true
fn_patching: ✔️
$
$ i use teleporter


A strange, electronic voice is projected into your mind:

  "Unusual setting detected!  Starting confirmation process!  Estimated time to completion: 1 billion years."

You wake up on a sandy beach with a slight headache.  The last thing you remember is activating that teleporter... but now you can't find it anywhere in your pack.  Someone seems to have drawn a message in the sand here:

    lVfSqITLZkYK

It begins to rain.  The message washes away.  You take a deep breath and feel firmly grounded in reality as the effects of the teleportation wear off.       

```

# 8/8 Beach


```
$ i look journal


Fireflies were using this dusty old journal as a resting spot until you scared them off.  It reads:

Day 1: We have reached what seems to be the final in a series of puzzles guarding an ancient treasure.  I suspect most adventurers give up long before this point, but we're so close!  We must press on!

Day 1: P.S.: It's a good thing the island is tropical.  We should have food for weeks!

Day 2: The vault appears to be sealed by a mysterious force - the door won't budge an inch.  We don't have the resources to blow it open, and I wouldn't risk damaging the contents even if we did.  We'll have to figure out the lock mechanism.

Day 3: The door to the vault has a number carved into it.  Each room leading up to the vault has more numbers or symbols embedded in mosaics in the floors.  
We even found a strange glass orb in the antechamber on a pedestal itself labeled with a number.  What could they mean?

Day 5: We finally built up the courage to touch the strange orb in the antechamber.  It flashes colors as we carry it from room to room, and sometimes the symbols in the rooms flash colors as well.  It simply evaporates if we try to leave with it, but another appears on the pedestal in the antechamber shortly thereafter.  It also seems to do this even when we return with it to the antechamber from the other rooms.

Day 8: When the orb is carried to the vault door, the numbers on the door flash black, and then the orb evaporates.  Did we do something wrong?  Doesn't the 
door like us?  We also found a small hourglass near the door, endlessly running.  Is it waiting for something?

Day 13: Some of my crew swear the orb actually gets heaver or lighter as they walk around with it.  Is that even possible?  They say that if they walk through certain rooms repeatedly, they feel it getting lighter and lighter, but it eventually just evaporates and a new one appears as usual.

Day 21: Now I can feel the orb changing weight as I walk around.  It depends on the area - the change is very subtle in some places, but certainly more noticeable in others, especially when I walk into a room with a larger number or out of a room marked '*'.  Perhaps we can actually control the weight of this mysterious orb?

Day 34: One of the crewmembers was wandering the rooms today and claimed that the numbers on the door flashed white as he approached!  He said the door still didn't open, but he noticed that the hourglass had run out and flashed black.  When we went to check on it, it was still running like it always does.  Perhaps he is going mad?  If not, which do we need to appease: the door or the hourglass?  Both?

Day 55: The fireflies are getting suspicious.  One of them looked at me funny today and then flew off.  I think I saw another one blinking a little faster than usual.  Or was it a little slower?  We are getting better at controlling the weight of the orb, and we think that's what the numbers are all about.  The orb starts at the weight labeled on the pedestal, and goes down as we leave a room marked '-', up as we leave a room marked '+', and up even more as we leave 
a room marked '*'.  Entering rooms with larger numbers has a greater effect.

Day 89: Every once in a great while, one of the crewmembers has the same story: that the door flashes white, the hourglass had already run out, it flashes black, and the orb evaporates.  Are we too slow?  We can't seem to find a way to make the orb's weight match what the door wants before the hourglass runs out.  If only we could find a shorter route through the rooms...

Day 144: We are abandoning the mission.  None of us can work out the solution to the puzzle.  I will leave this journal here to help future adventurers, though I am not sure what help it will give.  Good luck!
```

![beach](beach.svg)


The orb has an internal value. To find it's value, pick the orb, 

```
$ mem init
$ mem filter = 22
Selected 3 values
$ mem list
3952: 22 -> 22
4664: 22 -> 22
27432: 22 -> 22
Listed 3 values
```

Then go north, north (+ 4), the orb offset is 3952:

```
$ mem list
3952: 22 -> 26
```

I couldn't get the graph search to work, so I brute forced by standing in the room next to the vault door depicting a '-'

$ state diskload vault_puzzle.json
$ solver vault
$ $ mem set 3952 31
$ i east

```
As you approach the vault door, the number on the vault door flashes white!  The hourglass is still running!  It flashes white!  You hear a click from the vault door.  The orb evaporates out of hour hands.

== Vault ==
This vault contains incredible riches!  Piles of gold and platinum coins surround you, and the walls are adorned with topazes, rubies, sapphires, emeralds, opals, dilithium crystals, elerium-115, and unobtainium.

Things of interest here:
- mirror

There is 1 exit:
- leave

$ i take mirror


Taken.

What do you do?

$ i use mirror


You gaze into the mirror, and you see yourself gazing back.  But wait!  It looks like someone wrote on your face while you were unconscious on the beach!  Through the mirror, you see "iTodwMouiWIT" scrawled in charcoal on your forehead.

Congratulations; you have reached the end of the challenge!
```
