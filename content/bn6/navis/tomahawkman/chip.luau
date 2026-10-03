--!strict
-- TomahawkMan's own chip, ETomahwk (the original's `sub_80F18AC`, his
-- action table's entry 0xA): he raises his axe (navis/tomahawkman/axe) for
-- 17 ticks and swings it; 10 ticks in the blow falls with a camera shake:
-- a tomahawk strike (navis/tomahawkman/strike) on each panel ahead in his
-- row, to the field's edge and six at most, 5 ticks apart, each breaking
-- its panel. 30 ticks later the axe goes, and he recovers for 30. See
-- docs/engine/standard-chips.md, "Action 0x0A".

local axe = require("./axe")
local collision = require("../../rules/collision")
local link = require("../../lib/link_chips")
local strike = require("./strike")

export type State = {
    timer: number,
    -- His axe (AIAttackVars+0x30).
    axe: Object?,
}

local STATE = { timer = "u16", axe = "object" }

local RAISE, SWING, RECOVER = 0x0, 0x4, 0x8
local ANIM_RAISE, ANIM_SWING, ANIM_RECOVER = 0x11, 0x12, 7
local SOUND = asset.sound("bn6:etomahwk")

local RAISE_TICKS, SWING_TICKS, RECOVER_TICKS = 0x10, 0x1E, 0x1E
-- The timer value the blow falls at, and the camera's shake then.
local FALL_AT = 0x14
local SHAKE, SHAKE_TICKS = 2, 0x1E

-- The strikes: 5 ticks apart, slashes at navis, no spark; six at most.
local DELAY = 5
local MAX_STRIKES = 6

-- `sub_80F18CC`: the axe out; 16 ticks after the entry.
local function raise(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        me.anim = ANIM_RAISE
        axe.put_on(me, s)
        s.timer = RAISE_TICKS
    elseif link.run_out(s) then
        link.go(me, SWING)
    end
end

-- `sub_80F197A`: a strike on each panel ahead while on the field.
local function strikes(me: Object)
    local element, damage = me.attack_element, link.damage_word(me)
    for n = 1, MAX_STRIKES do
        local x, y = me.panel_x + me.facing * n, me.panel_y
        if not field.valid(x, y) then
            return
        end
        local spec = { delay = (n - 1) * DELAY, is = collision.slash, hits = collision.hits_navis, spark = nil }
        strike.spawn(me, x, y, element, spec, damage)
    end
end

-- `sub_80F1902`: the swing; the blow falls at 20 left; 30 ticks.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_SWING
        s.timer = SWING_TICKS
        local a = s.axe
        if a then
            a.action = axe.SWING
        end
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t == FALL_AT then
        battle.shake_camera(SHAKE, SHAKE_TICKS)
        battle.play_sound(SOUND)
        strikes(me)
    elseif t <= 0 then
        link.go(me, RECOVER)
    end
end

-- `sub_80F194C`: the axe goes; 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_RECOVER
        s.timer = RECOVER_TICKS
        local a = s.axe
        if a then
            a.action = axe.END
        end
    elseif link.run_out(s) then
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [RAISE] = raise, [SWING] = swing, [RECOVER] = recover }

return define.chip {
    id = "bn6:etomahwk",
    codes = { "T" },
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = { formula = "navi_level", base = 100, per_level = 30 },
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    library = { number = 406, index = 0, sort = 7 },
    slot_in_limit = 1,
    -- `sub_80F18AC` (`off_80F18C0`).
    action = define.action {
        id = "bn6:etomahwk/action",
        state = STATE,
        update = function(me: Object, s: State)
            link.run(me, s, PHASES, "ETomahwk (sub_80F18AC)")
        end,
    },
}
