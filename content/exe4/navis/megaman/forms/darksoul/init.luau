--!strict
-- The dark soul (soul 15, NaviStats +0x24): what a dark MegaMan becomes for 960 ticks when a hit would delete him,
-- once a battle (rules/souls/dark: its trigger, its ticks, and the auto battle AI that drives him in it). No chip
-- gives it and Double Soul never offers it: it is no soul of a family (no `soul`).
--
-- His sprite in it is his own (0x0800B90A adds nothing for soul 15: 00-00), in the palette 1, black, and his color's
-- (0x0800C018: rules/light_dark's `navi_palette`); he wears nothing (0x08018068's row 15). His emotion is found as
-- out of a soul (0x0800F4BA), so his face is his base form's by it: the worn-out one, his mood 0. DarkMega comes as
-- his image at the change (megaman-ds, 08-0f: 0x080E37D8's category 8 by the soul), holding nothing. His weapons are
-- 0x080184F0's row 15, his base form's (the buster and the charged shot, which the AI fires: no button of his does,
-- 0x0800BE28 skipping the B button's part of the input and 0x0800BBB2 the charge's count). His status reset
-- (0x0800E17E) leaves him untouchable (flag 0x08000000), which the revert's, in his base form, takes away.

local base = require("./base").base
local dark = require("../../../rules/souls/dark")
local soul = require("./soul")

local form: FormDef = {
    change = dark.change,
    revert = soul.revert,
    image = soul.image(asset.sprite("megaman-ds")),
    sprite = asset.sprite("megaman"),
    element = "null",
    buster_bonus = 0,
    weapons = base.weapons,
    traits = { "ignores_b", "emotions_as_base" },
    status_reset = { "untouchable" },
    palette = 1,
    mugshot = base.mugshot,
}

return { darksoul = form }
