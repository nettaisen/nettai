--!strict
-- EXE4's battle themes (the rules' `battle_themes`): the songs a player may pick as a battle's music in place of
-- its game's own, in this game's battles or another's, as a list shows them. Each is the music the original's battle
-- settings records start (their byte 3) for a kind of battle: the virus battles' (369 records), the boss battles'
-- (52, the navis met in the net), the tournaments' (24, at the three venues) and Duo's two (../compat/assets.toml).
-- Their names are the locales' (`battle_themes`, by asset name). Presentation: the battle plays none of them by this
-- list.

return {
    asset.sound("virus-battle"),
    asset.sound("boss-battle"),
    asset.sound("tournament-battle"),
    asset.sound("final-battle"),
}
