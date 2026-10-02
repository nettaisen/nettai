--!strict
-- HeatMan's own chip, HeatPres (the original's `sub_80F0778`, his action
-- table's entry 0xA): he jumps onto the panel three ahead (20 ticks of an
-- arc), and landing on solid ground presses down: a flame (chips/heatman/
-- flame) on each solid panel beside him (left, right, above, below) and a
-- hit on his own panel for 60 ticks. Then he is back on his panel. See
-- docs/engine/standard-chips.md, "Action 0x0A".

local collision = require("../../rules/collision")
local flame = require("../../chips/heatman/flame")
local link = require("../../lib/link_chips")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

export type State = { timer: number }

local STATE = { timer = "u16" }

local JUMP, PRESS, BACK = 0x0, 0x4, 0x8
local ANIM_JUMP = 0x12
local SOUND = asset.sound("land")

-- The jump: three panels ahead, landing at Z 0 in 20 ticks, falling by
-- 1.5 pixels a tick more each tick.
local JUMP_PANELS, JUMP_TICKS, GRAVITY = 3, 0x14, -0x1_8000

-- What he lands on to press, and where a flame can burn: a solid panel.
local SOLID = 0x10

-- The flames beside him (`byte_80F3824`: left, right, above, below, not
-- turned to his side), and theirs: they wait a dimming out and burn for 60
-- ticks, as attacks.
local BESIDE = { { -1, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } }
local FLAME: HeatFlame = { while_dimmed = false, ticks = 0x3C, is = collision.attack }

-- The press's hit on his panel, for 60 ticks.
local PRESS_TICKS = 0x3C
local PRESS_HIT_MOD = 3

-- `sub_80F0798`: the jump. Its entry aims the arc; each tick after moves
-- along it while the timer, counted down, stays at 0 or more. Then he is
-- on the ground, and presses on a solid panel.
local function jump(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.collision:set_region(nil)
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me.anim = ANIM_JUMP
        local x, y = field.center(me.panel_x + me.facing * JUMP_PANELS, me.panel_y)
        me.vel = trajectory.in_ticks(me.pos, x, y, 0, JUMP_TICKS, GRAVITY)
        s.timer = JUMP_TICKS
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        local v = me.vel
        me.pos = me.pos + v
        me.vel = Vec3.new(v.x, v.y, v.z + GRAVITY)
        me:set_panel_from_coordinates()
        return
    end
    -- Landed: Z's whole part 0 (its fraction stays).
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    link.go(me, if bit32.btest(field.flags(me.panel_x, me.panel_y), SOLID) then PRESS else BACK)
end

-- `sub_80F37D8`: a flame on each solid panel beside him. The registers its
-- spawner leaves (the row, the element, and what the panel lookup left in
-- r3: the row less 1) are each flame's position until its init.
local function flames(me: Object)
    local element, damage = me.attack_element, link.damage_word(me)
    for _, d in BESIDE do
        local x, y = me.panel_x + d[1], me.panel_y + d[2]
        if bit32.btest(field.flags(x, y), SOLID) then
            flame.spawn(me, x, y, element, FLAME, damage, Vec3.new(y, element, y - 1))
        end
    end
end

-- `sub_80F0832`: the press: the flames, and a hit on his panel lasting
-- 60 ticks (with its sound, if it came); 60 ticks.
local function press(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        flames(me)
        local hit = battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.attack_element,
            z = 0,
            region = regions.single,
            hit_effect = sparks.fire,
            target = collision.hits_navis,
            self_type = collision.attack,
            damage = link.damage(me),
            stamina = link.stamina(me),
            hit_mod = PRESS_HIT_MOD,
        })
        if hit then
            hit.timer = PRESS_TICKS
            battle.play_sound(SOUND)
        end
        s.timer = PRESS_TICKS
    elseif link.run_out(s) then
        link.go(me, BACK)
    end
end

-- `sub_80F0898`: back on his panel, his region back; idle.
local function back(me: Object, _s: State)
    me.anim = 0
    me.collision:set_region(regions.single)
    me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
    me:unreserve_panel(me.panel_x, me.panel_y)
    me:set_coordinates_from_panel()
    me:exit_attack()
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [JUMP] = jump, [PRESS] = press, [BACK] = back }

return define.chip {
    id = "heatpres",
    codes = { "H" },
    element = "fire",
    family = "fire",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = { formula = "navi_level", base = 50, per_level = 20 },
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    library = { number = 370, index = 0, sort = 0 },
    slot_in_limit = 1,
    -- `sub_80F0778` (`off_80F078C`).
    action = define.action {
        id = "heatpres/action",
        state = STATE,
        update = function(me: Object, s: State)
            link.run(me, s, PHASES, "HeatPres (sub_80F0778)")
        end,
    },
}
