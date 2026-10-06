--!strict
-- MegaMan's base form.

local buster = require("../weapons/buster")
local charged_shot = require("../weapons/charged_shot")

return define.form {
    id = "base",
    -- EXE6's base form.
    base = true,
    sprite = asset.sprite("megaman"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    palette = 0,
    -- MegaMan's faces in the emotion window, by his emotion (`byte_801E6F4`).
    mugshot = {
        normal = asset.mugshot("megaman"),
        angry = asset.mugshot("megaman-angry"),
        tired = asset.mugshot("megaman-tired"),
        full_synchro = asset.mugshot("megaman-full-synchro"),
        worn_out = asset.mugshot("megaman-worn-out"),
    },
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 0 },
}
