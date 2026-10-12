--!strict
-- RockCube's and IceCube's dimming controller (EXE6's effect object #0x37, `sub_80E4634`; EXE5's the same code,
-- 0x080E4664; EXE4's #0x37, 0x080E60AC): the usual dimming phases, with an effect that places a rock on the panel in
-- front of the user and waits 60 ticks. What differs between the chips is the rock (the original's chip parameters:
-- its variant, its field-object class and its entrance); between the games, their rock, its place sound, where the
-- controller spawns and what of its user's side and facing it takes (a maker taking a game's look).

local dimming_chips = require("@exelib/dimming")

local cube = {}

export type State = {
    -- The rock it places: its variant, class and entrance.
    variant: RockVariant,
    class: number,
    entrance: RockEntrance,
}

export type CubeLook = {
    -- The controller's kind id.
    id: string,
    -- The game's rock (`sub_80CFBC4`).
    spawn_rock: (x: number, y: number, side: number, spec: RockSpec, damage: number) -> Object?,
    place_sound: Sound,
    -- Where the controller spawns, by its user and the dimming (none: where `dimming_chips.spawn` puts it).
    position: ((user: Object, spec: DimmingChipSpec) -> Vec3)?,
    -- Its spawn copies the user's side alone, not its facing (`dimming_chips.spawn`'s `side_only`: EXE4's
    -- 0x080E612C, whose effect then places the rock by its side's way, 0x080E60F8's object_getEnemyDirection, which
    -- the unflipped controller faces); none: the side and the facing (EXE6's and EXE5's, whose effect places it by
    -- the facing, object_getFrontDirection).
    side_only: boolean?,
}

export type Cube = {
    kind: Kind,
    -- A chip's `dimming` hook (`sub_80E46B6`, `off_802CCB4[6]`): the controller that places a rock of `placed`.
    places: (placed: RockSpec) -> (user: Object, spec: DimmingChipSpec) -> Object?,
}

local WAIT = 0x3C

function cube.make(look: CubeLook): Cube
    -- `sub_80E4678`: a rock in front of the user, hitting when thrown with the chip's damage word; then 60 ticks.
    local function effect(me: Object)
        if me.phase_init == 0 then
            local s = me.state :: State
            local placed: RockSpec = { variant = s.variant, class = s.class, entrance = s.entrance }
            look.spawn_rock(me.panel_x + me.facing, me.panel_y, me.alliance, placed, dimming_chips.damage_plus(me, 0))
            battle.play_sound(look.place_sound)
            me.timer = WAIT
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t <= 0 then
            dimming_chips.done(me)
        end
    end

    local kind = new.kind {
        id = look.id,
        pool = "effect",
        state = {
            variant = "record:rock-variant",
            class = "u8",
            entrance = { "rise", "instant", "fall", "placed" },
        },
        -- `sub_80E4634`.
        update = dimming_chips.phases { name = "RockCube", effect = effect },
    }

    local function places(placed: RockSpec): (user: Object, spec: DimmingChipSpec) -> Object?
        return function(user: Object, spec: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(kind, user, spec, look.side_only)
            if o then
                if look.position then
                    o.pos = look.position(user, spec)
                end
                local s = o.state :: State
                s.variant, s.class, s.entrance = placed.variant, placed.class or 0, placed.entrance or "rise"
            end
            return o
        end
    end

    return { kind = kind, places = places }
end

return cube
