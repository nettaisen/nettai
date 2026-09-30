--!strict
-- Registration by number, until chips are handles
-- (docs/design/content-model-v2.md §12, step 3b): the sword chips' records
-- name the original's action 0x13 and their subtype, and registration by
-- number runs this module for them (and for the variable swords' picks).
-- It runs the chip's own slash (chips/sword, chips/wideswrd, ...), picked
-- by the subtype (and StepSwrd's first attack parameter) as the original's
-- routine branches. It goes when the chips' definitions replace their
-- records.

local slash = require("../../lib/swords/slash")
local sword = require("../sword/chip")
local wideswrd = require("../wideswrd/chip")
local longswrd = require("../longswrd/chip")
local wideblde = require("../wideblde/chip")
local longblde = require("../longblde/chip")
local lifesrd = require("../lifesrd/chip")
local drksword = require("../drksword/chip")
local muramasa = require("../muramasa/chip")
local ftrsword = require("../ftrsword/chip")
local crosswrd = require("../crosswrd/chip")
local dbldream = require("../dbldream/chip")
local fireswrd = require("../fireswrd/chip")
local aquaswrd = require("../aquaswrd/chip")
local elecswrd = require("../elecswrd/chip")
local bambswrd = require("../bambswrd/chip")
local stepswrd = require("../stepswrd/chips")

local legacy = {
    state = slash.STATE,
}

-- The chips' slashes by subtype (StepSwrd shares WideSwrd's).
local BY_SUBTYPE: { [number]: Action } = {
    [0x0] = sword.action,
    [0x1] = wideswrd.action,
    [0x2] = longswrd.action,
    [0x3] = wideblde.action,
    [0x4] = longblde.action,
    [0x5] = lifesrd.action,
    [0x6] = drksword.action,
    [0x8] = muramasa.action,
    [0x9] = ftrsword.action,
    [0xA] = crosswrd.action,
    [0xB] = dbldream.action,
    [0xC] = fireswrd.action,
    [0xD] = aquaswrd.action,
    [0xE] = elecswrd.action,
    [0xF] = bambswrd.action,
}

local function action_of(me: Object): Action
    if me:attack_param(1) ~= 0 then
        return stepswrd[1].action
    end
    local a = BY_SUBTYPE[me.variant]
    if not a then
        error(string.format("sword subtype %#x has no chip", me.variant))
    end
    return a
end

function legacy.update(me: Object, s: any)
    action_of(me).update(me, s)
end

return legacy
