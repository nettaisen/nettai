--!strict
-- MegaMan's base form.

local buster = require("../../weapons/buster/weapon")
local charged_shot = require("../../weapons/charged-shot/weapon")

return define.form {
    id = "bn6:base",
    kind = "base",
    sprite = asset.sprite("bn6:megaman"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    palette = 0,
    -- MegaMan's faces in the emotion window, by his emotion (`byte_801E6F4`).
    mugshot = {
        normal = asset.mugshot("bn6:megaman"),
        angry = asset.mugshot("bn6:megaman-angry"),
        tired = asset.mugshot("bn6:megaman-tired"),
        full_synchro = asset.mugshot("bn6:megaman-full-synchro"),
        worn_out = asset.mugshot("bn6:megaman-worn-out"),
    },
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 0 },
}
