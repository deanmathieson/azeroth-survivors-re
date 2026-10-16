AZEROTH SURVIVORS - LAUNCH NIGHT KIT
====================================

You need a WoW 1.12.1 client's Data folder (the one with the 14 .MPQ files).
This kit does NOT include it. The full guide is docs/FRIENDS.md in the repo; the short version:

1. Unzip this whole folder somewhere you can write to (Desktop or Documents is fine,
   not Program Files). Spaces in the path are fine.

2. Double-click one of:
     Play.bat   - start the game.
     Host.bat   - start the game and host a party (UDP port 5000 by default).
     Join.bat   - start the game and join a host (type their address, e.g. 100.64.0.7:5000).
   The first time, it asks where your WoW "Data" folder is and remembers it in
   wow-data-path.txt. Wrong folder? Delete that file and run again.

3. Host: when everyone shows in the Party list, click Start Run.
   You can also skip the .bat helpers: in the menu click Multiplayer, then Host or Join.

EVERYONE MUST USE THE SAME ZIP. The file name ends in a build code
(AzerothSurvivors-xxxxxxx.zip); if yours differs from the host's, get the latest one.

Your profile (settings, talents, achievements) lives in the benilla-config folder here.
To update to a new kit, copy your old benilla-config folder over the new one.

TROUBLESHOOTING
- "Windows protected your PC": click More info, then Run anyway. (It's unsigned, not evil.)
- Firewall prompt when hosting: allow on Private networks.
- "different build" when joining: you and the host have different zips. Get the latest.
- Can't connect: both of you open Tailscale and check you're on the same tailnet; use the
  host's 100.x.y.z address; the host must allow the game through the Windows firewall.
- The game closed with an error: crash notes are in benilla-config\Diagnostics. Send them over.

No Blizzard files are in this kit. World of Warcraft is a trademark of Blizzard Entertainment;
this is an unofficial fan project and is not affiliated with or endorsed by Blizzard.
