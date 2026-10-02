--!strict
-- EraseMan's charged attack (`eraseman/charge`): the original's weapon
-- routine 0x47 (`sub_8012292`) and his action table's entry 9
-- (`sub_80F0FB4`). He draws his scythe back (20 ticks) and swings it (10
-- ticks): on the swing's 5th tick a slash over the column two panels
-- ahead, with its effect; then 30 ticks.

local collision = require("../../rules/collision")
local link = require("../../lib/link_chips")
local regions = require("../../lib/regions")
local weapon = require("../../lib/weapon")

export type State = { timer: number }

local charge = {}

local DRAW, SWING, RECOVER = 0x0, 0x4, 0x8
local ANIM_DRAW, ANIM_SWING = 0x13, 0x14
local DRAW_TICKS, SWING_TICKS, RECOVER_TICKS = 0x14, 0xA, 0x1E
-- The swing's timer when the slash lands.
local SLASH_AT = 5

local SOUND = asset.sound("bn6:spin")

-- The hits' counter byte, and the slash's element: Null (the attack's
-- element byte).
local HIT_PARAM = 0x8A
local NULL = 0

-- The slash: how far ahead, its hit's modifier, and its effect (the
-- swords' wide slash in his colour) 24 pixels up.
local AHEAD = 2
local HIT_MOD = 3
local SLASH = define.effect { sprite = asset.sprite("bn6:sword-slash"), anim = 0, palette = 6 }
local SLASH_Z = 0x18_0000

-- `sub_8012292`: 50 damage and 20 more per point of buster Attack (up to
-- 5).
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x32, 0x14)
    navi.hit_param = HIT_PARAM
    navi.attack_element = NULL
    return charge.action
end

-- `sub_80F10F2`, `sub_80F111C`: the slash's hit on the column two panels
-- ahead, and its effect there.
local function slash(me: Object)
    local x, y = me.panel_x + AHEAD * me.facing, me.panel_y
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.attack_element,
        z = 0,
        region = regions.wide,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.slash,
        damage = link.damage(me),
        stamina = link.stamina(me),
        hit_mod = HIT_MOD,
    })
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, SLASH_Z), SLASH, link.flip(me))
end

-- `sub_80F0FD4`: he draws back; 20 ticks.
local function draw(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_DRAW
        s.timer = DRAW_TICKS
    elseif link.run_out(s) then
        link.go(me, SWING)
    end
end

-- `sub_80F0FF8`: the swing; 10 ticks, the slash at 5 left.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_SWING
        battle.play_sound(SOUND)
        s.timer = SWING_TICKS
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t == SLASH_AT then
        slash(me)
    elseif t <= 0 then
        link.go(me, RECOVER)
    end
end

-- `sub_80F1032`: 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        s.timer = RECOVER_TICKS
    elseif link.run_out(s) then
        me.anim = 0
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [DRAW] = draw, [SWING] = swing, [RECOVER] = recover }

charge.action = define.action {
    id = "bn6:eraseman/charge/action",
    state = { timer = "u16" },
    -- `sub_80F0FB4` (`off_80F0FC8`).
    update = function(me: Object, s: State)
        link.run(me, s, PHASES, "EraseMan's charge (sub_80F0FB4)")
    end,
}

charge.weapon = define.weapon {
    id = "bn6:eraseman/charge",
    -- The charge table's row 0x47, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 60, 55, 50, 45, 40, 120, 110, 100 },
    setup = setup,
}

return charge
