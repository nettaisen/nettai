--!strict
-- RockCube's and IceCube's dimming controller (the original's effect
-- object #0x37, `sub_80E4634`): the usual dimming phases, with an effect
-- that places a rock (./rock) on the panel in front of the user and
-- waits 60 ticks. What differs between the chips is the rock (the
-- original's chip parameters: its variant, its field-object class and its
-- entrance).

local dimming_chips = require("@exelib/dimming")
local rock = require("./rock")

local cube = {}

export type State = {
    -- The rock it places: its variant, class and entrance.
    variant: RockVariant,
    class: number,
    entrance: RockEntrance,
}

local PLACE_SOUND = asset.sound("place")
local WAIT = 0x3C

-- `sub_80E4678`: a rock in front of the user, hitting when thrown with the
-- chip's damage word; then 60 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        local s = me.state :: State
        local placed: RockSpec = { variant = s.variant, class = s.class, entrance = s.entrance }
        rock.spawn(me.panel_x + me.facing, me.panel_y, me.alliance, placed, dimming_chips.damage_plus(me, 0))
        battle.play_sound(PLACE_SOUND)
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        dimming_chips.done(me)
    end
end

local KIND = define.kind {
    id = "rockcube/cube",
    pool = "effect",
    state = {
        variant = "record:rock-variant",
        class = "u8",
        entrance = { "rise", "instant", "fall", "placed" },
    },
    -- `sub_80E4634`.
    update = dimming_chips.phases { name = "RockCube", effect = effect },
}

-- A chip's `dimming` hook (`sub_80E46B6`, `off_802CCB4[6]`): the
-- controller that places a rock of `placed`.
function cube.places(placed: RockSpec): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(KIND, user, spec)
        if o then
            local s = o.state :: State
            s.variant, s.class, s.entrance = placed.variant, placed.class or 0, placed.entrance or "rise"
        end
        return o
    end
end

return cube
