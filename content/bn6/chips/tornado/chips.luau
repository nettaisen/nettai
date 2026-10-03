--!strict
-- Tornado and Static (the original's action 0x2F, `sub_80ED454`): the navi
-- holds out a fan (an attachment) and, ten ticks in, raises tornadoes
-- (tornado.luau) two panels ahead; the fan turns while they blow. Tornado
-- raises one on that panel; Static raises them on more panels the more
-- kinds of NaviCust bug the navi has (none: that panel; up to three: also
-- the one behind it, the two beside it, and the two beside that one), a
-- static tornado each. Another game's chips take the blow (`chips.blow`)
-- with their fan and tornado kind. The action's third form (its subtype 3, which no
-- chip has) does the same by a side statistic (`sub_800AB3A`, counter 3),
-- spreading back toward the navi, with HP-bug tornadoes, and a fan of
-- another palette. 42 ticks, then 21 to put the fan away.

local attachment = require("../../objects/attachment/attachment")
local lockon = require("../../rules/lockon")
local tornado = require("./tornado")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- What a chip's blow raises: one tornado (`single`), or one on each panel
-- of `spread` at a level, by the navi's kinds of NaviCust bug or by its
-- side's statistic 3 (`by`); and the fan it holds, with an addition to its
-- palette.
export type Blow = {
    -- The action's key when no chip definition holds it.
    id: string?,
    fan: AttachmentLook,
    fan_palette: number?,
    -- The tornadoes' kind (BN6's by default).
    tornado_kind: Kind?,
    single: TornadoSpec?,
    -- (`tornadoes`, by level, over `tornado`: another game's tornadoes
    -- that differ by level, BN5's Static's.)
    spread: {
        by: "bug_kinds" | "side_stat",
        panels: { [number]: { { number } } },
        tornado: TornadoSpec,
        tornadoes: { [number]: TornadoSpec }?,
    }?,
}

-- One state for the action's blows.
local STATE = { timer = "u16" }

-- The phases, on the attack's step.
local BLOW, PUT_AWAY = 0x0, 0x4
-- The tick (timer value) the tornadoes rise.
local RISE_AT = 0x20

-- The fan (the original's attachment rows 0x21 and 0x22).
local FAN = attachment.look { sprite = asset.sprite("bn6:hand-fan"), palette = 0, attach_point = 9 }
local CHARGED_FAN = attachment.look { sprite = asset.sprite("bn6:hand-fan"), palette = 1, attach_point = 9 }

-- The panels each spread covers, by level (capped at 3): Static's
-- (`off_80CA160`) and the action's third form's (`off_80CA1F8`), { dx, dy }
-- from the panel two ahead, dx toward the navi's front.
local STATIC_SPREAD = {
    [0] = { { 0, 0 } },
    { { 0, 0 }, { 1, 0 } },
    { { 0, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } },
    { { 0, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 }, { 1, -1 }, { 1, 1 } },
}
local BACK_SPREAD = {
    [0] = { { 0, 0 } },
    { { 0, 0 }, { -1, 0 } },
    { { 0, 0 }, { -1, 0 }, { 0, -1 }, { 0, 1 } },
    { { 0, 0 }, { -1, 0 }, { 0, -1 }, { 0, 1 }, { 1, 0 } },
}
-- The side statistic the third form spreads by.
local SIDE_STAT = 3

-- Turn the fan (its requested animation by `step`). Without one (the pool
-- was full) the game writes to address 0x10, which does nothing.
local function turn_fan(me: Object, step: number)
    local fan = me.overlay
    if fan then
        fan.anim = fan.anim + step
    end
end

-- A chip's blow.
local function blow(spec: Blow): Action
    -- `sub_80CA104` / `sub_80CA19E`: the spread's tornadoes around (x, y).
    local function spread_tornadoes(me: Object, x: number, y: number, element: number, damage: number)
        local spread = spec.spread :: any
        local level = if spread.by == "bug_kinds"
            then battle.navi(me.alliance).bug_kinds
            else battle.side_stat(me.alliance, SIDE_STAT)
        local dir = me.facing
        level = math.min(level, 3)
        local by_level = spread.tornadoes :: { [number]: TornadoSpec }?
        local t = if by_level then by_level[level] else spread.tornado
        for _, p in spread.panels[level] do
            tornado.spawn(me, x + dir * p[1], y + p[2], element, damage, t, spec.tornado_kind)
        end
    end

    -- `sub_80ED470`: hold out the fan; on the timer's 0x20 raise the
    -- tornadoes and turn the fan.
    local function blowing(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(0x0A)
            me:open_counter_window()
            attachment.attach(me, spec.fan, "overlay", { palette_add = spec.fan_palette })
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = 0x2A
        end
        if s.timer == RISE_AT then
            local x, y = me.panel_x + 2 * me.facing, me.panel_y
            -- The damage word (the hit parameter in its high half), plus
            -- the Atk+ bonus.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            local single = spec.single
            if single then
                tornado.spawn(me, x, y, me.attack_element, damage, single, spec.tornado_kind)
            else
                spread_tornadoes(me, x, y, me.attack_element, damage)
            end
            turn_fan(me, 1)
        end
        local t = s.timer - 1
        s.timer = t
        if t <= 0 then
            me.step = PUT_AWAY
            me.step_init = 0
        end
    end

    -- `sub_80ED52C`: turn the fan back; 21 ticks later, back to idle.
    local function put_away(me: Object, s: State)
        if me.step_init == 0 then
            turn_fan(me, -1)
            s.timer = 0x14
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end

    -- `sub_80ED454`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == BLOW then
            blowing(me, s)
        elseif step == PUT_AWAY then
            put_away(me, s)
        else
            error(string.format("Tornado phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = STATE, args = spec, update = update }
end

local chips = {
    define.chip {
        id = "bn6:tornado",
        codes = { "L", "R", "T" },
        element = "null", family = "wind", class = "standard",
        rarity = 2, mb = 16, damage = 20, hit_param = 4, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 53, index = 53, sort = 442 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.firebrn1 },
        action = blow {
            fan = FAN,
            single = { variant = tornado.variants.plain, status = nil, ticks = 0x20 },
        },
    },
    define.chip {
        id = "bn6:static",
        codes = { "G", "S", "V" },
        element = "null", family = "wind", class = "standard",
        rarity = 2, mb = 30, damage = 20, hit_param = 4, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 54, index = 54, sort = 420 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.firebrn1 },
        action = blow {
            fan = CHARGED_FAN,
            spread = {
                by = "bug_kinds",
                panels = STATIC_SPREAD,
                tornado = { variant = tornado.variants.static, status = nil, ticks = 0x20 },
            },
        },
    },
}

-- The action's third form, which no chip has (the original's subtype 3,
-- with `byte_80CA208`'s tornadoes).
chips.back_spread = blow {
    id = "bn6:tornado/back-spread",
    fan = CHARGED_FAN,
    fan_palette = 1,
    spread = {
        by = "side_stat",
        panels = BACK_SPREAD,
        tornado = { variant = tornado.variants.hp_bug, status = nil, ticks = 0x20 },
    },
}

chips.STATE = STATE
-- The blow, the spreads and the fans, for another game's chips.
chips.blow = blow
chips.STATIC_SPREAD, chips.BACK_SPREAD = STATIC_SPREAD, BACK_SPREAD

return chips
