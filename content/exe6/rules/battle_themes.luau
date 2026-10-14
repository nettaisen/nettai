--!strict
-- EXE6's battle themes (the rules' `battle_themes`): the songs a player may pick as a battle's music in place of
-- its game's own, in this game's battles or another's, as a list shows them. Each is the music the original's battle
-- settings records start (their byte 2) for a kind of battle: the virus battles' (`BattleSettingsList0`'s 201
-- records), the boss battles' (its 65 with a navi), the two against a Cybeast (records 0x84 and 0x85, NameIDs 0x179
-- and 0x17B in Falzar, 0x173 and 0x175 in Gregar: the disassembly's "final battle") and the crossover battle's
-- (record 0, the disassembly's name). Their names are the locales' (`battle_themes`, by asset name). Presentation:
-- the battle plays none of them by this list.

return {
    asset.sound("virus-battle"),
    asset.sound("boss-battle"),
    asset.sound("final-battle"),
    asset.sound("crossover-battle"),
}
