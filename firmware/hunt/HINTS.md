# TENGUCON 3.0 hunt: desk hints

Hint 1 when a team asks. Hint 2 only if they are still stuck half an hour later.
Never say a code out loud. If a team is right but typed it wrong, tell them to check spaces and dashes.
After a wrong code, the badge ignores OK for 5 seconds.

## Codes

| Stage | Where it comes from | Code |
|-|-|-|
| 1 | USB serial, base64, every 10 s | `yamabushi-boots-b6mhm73w` |
| 2 | Morse on the white LED | `long nose blinks hw6wgq` |
| 3 | NVS `hunt/flag`, XOR board name | `hauchiwa-in-flash-h6jk7pqk` |
| 4 | crackme in NVS `hunt/gift`, maze path `RRRDDRDRRDDDAUUULLULUUADDRDARRRDDDA` | `kurama-dojo-mpp3esnn` (also the TENGU-HUNT password) |
| 5 | broadcast + sticker | `sojobo-fcq9dc-xqlg74y6nvbabz` |
| 6 | Badge swap, 5 badges | `six-long-noses-48w3bh` |
| 7 | Myojin remote token + stone name | `pdky8fu7y-hachigoro` (also `-hachigorou`, `-garappachi`) |
| 8 | token + stairs year, opens the finish screen | `bvhrcnsexa-1935` |

Wi-Fi box, on the hidden network TENGU-HUNT:

- Broadcast to UDP port 31337 every few seconds: `next code: sojobo-fcq9dc- and 14 more chars. they are on me.`
- Sticker on the box: `xqlg74y6nvbabz`

## Stage 1: the hidden start (no prompt on the badge)

The badge prints the first code as base64 on USB serial every 10 seconds, mixed into the boot log.

Say nothing for the first hour. An empty "Enter code:" box is the puzzle.

- Hint 1: "The badge talks all the time. You just aren't listening yet."
- Hint 2: "Plug it into your laptop over USB-C and open the serial port. Baud rate doesn't matter. One line keeps coming back every 10 seconds and it isn't a log line."

Nobody has the first code after 2 hours? Post hint 1 on the con chat. Still nobody at 3 hours, post hint 2.

## Stage 2: Morse on the white LED

- Hint 1: "One LED is white and it never stops. Film it and play it back slowly."
- Hint 2: "Count frames. 1 unit is 200 ms: a dot is 1 unit, a dash is 3. The answer has spaces in it, and you type them."

## Stage 3: XORed blob in NVS

- Hint 1: "`esptool read-flash` dumps everything. The partition table at 0x8000 tells you where nvs lives."
- Hint 2: "ESP-IDF ships nvs_tool.py (components/nvs_flash/nvs_partition_tool). It prints every namespace and key. The XOR key is the board name from the menu header without the '// ', same case, like NeonTengu3f."

## Stage 4: the crackme

- Hint 1: "The gift is an ELF, the blob in namespace hunt, key gift. Pull it out and open it in Ghidra (Xtensa). Each key you press goes through a loop that reads one byte, picks what to do with it, and loops back: that's a tiny CPU. Its program sits in .rodata, and each byte is XORed with something that depends on where it is."
- Hint 2: "The CPU has 8 instructions. Emulate it and you'll see it read a 64-byte table as an 8x8 maze: low 4 bits are walls, high 4 bits are numbered squares. Arrows move one square, A only counts on the numbered squares in order. The badge wants the shortest path, every key and nothing extra. Don't brute force it: each try costs about a second."

## Stage 5: Wi-Fi fox hunt

- Hint 1: "Open Wi-Fi in the main menu, pick hidden network, type TENGU-HUNT. The crackme code is the password. Once you are on, the dBm number is a thermometer: closer to zero is warmer."
- Hint 2: "The box is on the con floor (say which one). It has a sticker with 14 chars. Read them and leave the box alone."

## Stage 6: IR swap

Badge swap counts names on every badge, but it only shows the code once the team has typed the Wi-Fi code in the hunt app. No code on their screen means they skipped stage 5. Desk badges work without any code.

- Hint 1: "Badge swap is in the main menu. Your badge needs to see 5 other badges on that screen. They don't all have to be there at once. Ask people, or sit by the desk where we keep 5 running."
- Hint 2: "Hold the badges close with the IR ends facing, and give each one a few seconds. Name not showing up? Closer, and stop shaking."

## Stage 7: Myojin remote and Kanda Myojin

The Myojin remote shows up in the IR blaster only once the team has typed the swap code. A reboot doesn't undo it.

- Hint 1: "One badge fires, a different badge listens in IR blaster > Learn. Write down each code in order, then read every hex pair as one ASCII character."
- Hint 2: "At the shrine, find the stone for Zenigata Heiji, the detective who throws coins. A smaller stone right next to it belongs to his sidekick. Google Lens reads the kanji fine."

The badge accepts hachigoro, hachigorou and garappachi as the name.

## Stage 8: mAAch ecute

- Hint 1: "It's the old red-brick Manseibashi station by the river, 10 minutes from UDX. The stairs have names, and the signs are by the stairs."
- Hint 2: "You go up one set of stairs and down the other. The code wants the number in the name of the second one."

## Black badge check at the desk

1. Run `python3 hunt/verify_finish.py <board name> <code>` and check that the board name on the screen matches the badge in their hand.
2. Write down the time. The first 3 valid badges win.
3. Ask them how they solved every challenge, one by one. No clear answer, no badge.
4. Ask 2 of these. A team that actually walked the route answers in seconds:
   - "What color is the coin-throwing detective's stone, and roughly how tall is it next to you?" (Scout the answer first.)
   - "At mAAch ecute, what did you see out the window up on the old platform?" (Expected: Chuo Line trains passing right next to you.)
   - "Which river does the old station sit on?" (Kanda River.)

Valid code but no idea what the walk looked like? They brute forced the year. Send them to go see it. The clock keeps running.
