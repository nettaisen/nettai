--!strict
-- BN5's MegaMan's base form (`bn5:base`, the engine's base form of BN5's game: rules-in-luau.md P1c): MegaMan in no
-- soul (NaviStats +0x2C 0), his sprite by his soul (0x0800DA58[0]). His weapons in it are his navi's, from his stats.

local buster = require("../weapons/buster")
local charged_shot = require("../weapons/charged-shot")

return define.form {
    id = "bn5:base",
    base = true,
    sprite = asset.sprite("bn5:megaman"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    palette = 0,
    -- MegaMan's faces in the emotion window, by his emotion (0x0801AF64: the table 0x0801AFB4 by BN5's emotion,
    -- 0x08012740; each brings its box): worried for a mood under 65, the dark face for a mood of 0 (a dark
    -- MegaMan's); and Hub Style's (NaviStats +0x4C, the navi's `hub_style`: 0x0801AF8E adds 11).
    mugshot = {
        normal = asset.mugshot("bn5:megaman"),
        angry = asset.mugshot("bn5:megaman-angry"),
        worried = asset.mugshot("bn5:megaman-worried"),
        full_synchro = asset.mugshot("bn5:megaman-full-synchro"),
        worn_out = asset.mugshot("bn5:megaman-dark"),
        variant = {
            normal = asset.mugshot("bn5:megaman-hub"),
            angry = asset.mugshot("bn5:megaman-hub-angry"),
            worried = asset.mugshot("bn5:megaman-hub-worried"),
            full_synchro = asset.mugshot("bn5:megaman-hub-full-synchro"),
            worn_out = asset.mugshot("bn5:megaman-hub-dark"),
        },
    },
    buster_arm = { anim = 0 },
}
