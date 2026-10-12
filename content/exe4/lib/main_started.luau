--!strict
-- Event flag 0x1187 (0x080406A4 reads it), as every netbattle has it: clear. It is the main subsystem's (0x080406B0
-- sets it as it starts, a game's load clears it), and a netbattle starts from a loaded save; the lab's netbattles have
-- it clear (chips/0x095-atkplus10, 0x0cd-atkplus30: the plus chips' sparkle; chips/0x01c-lifesync: LifeSync's link
-- battle wait). EXE5 tests its operation battle where EXE4 tests it (lib/plus, LifeSync's controller).
return false
