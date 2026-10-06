--!strict
-- EXE5's BodyGrd (the dimming handler's subtype 46), where EXE6's is a trap: its controller (effect object #0x6D,
-- 0x080E8620: the usual dimming phases; its effect, 0x080E8664, sends out the striker at once with the controller's
-- element and damage word plus the Atk+ bonus, and is done when its second parameter is 0, the record's: at once),
-- the shared striker (@exelib/bodygrd/striker, effect object #0x6E, 0x080E86B0: EXE6's code) with EXE5's identity and
-- EXE5's diving shuriken (chips/shadoman/shuriken, attack object #0x5C).

local dimming_chips = require("@exelib/dimming")
local shuriken = require("../shadoman/shuriken")
local striker = require("@exelib/bodygrd/striker")

local bodygrd = {}

bodygrd.striker = striker.make_kind {
    id = "bodygrd/striker",
    -- NameID 0xD3: a field object nothing swallows, leaves as junk or throws (Poltrgst's list).
    identity = new.identity {
        class = "field_object",
        version = 0,
        ai_index = 1,
        scrap = false,
        throwable = false,
        look = { anim = 0, palette = 0, shadow = false },
    },
    shuriken = shuriken.kind,
    throw_sound = asset.sound("boomerang"),
}

export type State = {
    -- The Atk+ bonus (+0x32): it adds to the striker's damage word.
    bonus: number,
    -- The record's second parameter (Param2): the effect waits for it to be 0.
    wait: number,
}

-- 0x080E8664: the striker out (0x080E87DE), then done once Param2 is 0.
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        striker.spawn(bodygrd.striker, me, me.element, dimming_chips.damage_plus(me, s.bonus))
        me.phase_init = 4
    end
    if s.wait == 0 then
        dimming_chips.done(me)
    end
end

bodygrd.controller = new.kind {
    id = "bodygrd/controller",
    pool = "effect",
    state = { bonus = "u16", wait = "u8" },
    -- 0x080E8620.
    update = dimming_chips.phases { name = "BodyGrd", effect = effect },
}

-- 0x080E868C: the chip's `dimming` hook; `wait`: the record's second parameter.
function bodygrd.hook(wait: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(bodygrd.controller, user, spec, true)
        if o then
            local s = o.state :: State
            s.bonus, s.wait = spec.bonus, wait
        end
        return o
    end
end

return bodygrd
