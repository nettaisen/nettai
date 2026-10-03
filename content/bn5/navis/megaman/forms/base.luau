--!strict
-- BN5's MegaMan's base form (`bn5:base`, the engine's base form of BN5's game: rules-in-luau.md P1c): MegaMan in no
-- soul (NaviStats +0x2C 0), his sprite by his soul (0x0800DA58[0]). His weapons in it are his navi's, from his stats.

local buster = require("../weapons/buster")
local charged_shot = require("../weapons/charged-shot")

return define.form {
    id = "bn5:base",
    kind = "base",
    sprite = asset.sprite("bn5:megaman"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    palette = 0,
    -- MegaMan's faces in the emotion window, by his emotion (0x0801AFB4's: BN6's order; each brings its box). Not
    -- yet: his dark faces (pictures 11-15, the local side's NaviStats +0x4C) and their palettes.
    mugshot = {
        normal = asset.mugshot("bn5:megaman"),
        angry = asset.mugshot("bn5:megaman-angry"),
        tired = asset.mugshot("bn5:megaman-tired"),
        full_synchro = asset.mugshot("bn5:megaman-full-synchro"),
        worn_out = asset.mugshot("bn5:megaman-worn-out"),
    },
    buster_arm = { anim = 0 },
}
