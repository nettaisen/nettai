--!strict
-- Registration by number (docs/design/content-model-v2.md §5.2, "As
-- built"): a record that names the original's action 0x13 runs this
-- module (as do the variable swords' picks), which runs the chip's own
-- slash (chips/sword, chips/wideswrd, ...) picked by the subtype (and
-- StepSwrd's first attack parameter) as the original's routine branches.
-- A chip content defines runs its action without it; the records that
-- reach it are the sword chips other records or the ruleset name by
-- number (the Program Advances', the dark chip, the swords SlashCross
-- charges, the Cross special's), Muramasa, ProtoMan's StepSwrd and the
-- test content's blades. It goes when those are definitions or roles
-- (step 5, phase C).

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
local stepswrd = require("../stepswrd/chip")
local protoman_stepswrd = require("../stepswrd/protoman")

local by_number = {
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
        -- ProtoMan's own StepSwrd is chip 0x19A.
        return if me.chip == 0x19A then protoman_stepswrd.action else stepswrd.action
    end
    local a = BY_SUBTYPE[me.variant]
    if not a then
        error(string.format("sword subtype %#x has no chip", me.variant))
    end
    return a
end

function by_number.update(me: Object, s: any)
    action_of(me).update(me, s)
end

return by_number
