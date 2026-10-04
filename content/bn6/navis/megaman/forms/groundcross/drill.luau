--!strict
-- GroundCross's charged shot (`groundcross/drill`): the original's weapon
-- routine 0x19 (`sub_8011D74`) and action 0x4D (`sub_80EEFDC`). The navi
-- burrows (its body leaves the field 3 ticks in), comes up on the panel
-- Beast Out lock-on mode 0xC picks in front of the first enemy in its row
-- (or of the other side's front column there), drills there with a drill
-- arm (DrilArm's drill, chips/drilarm/drill) for 24 ticks and waits 10 more, then burrows back to
-- the panel it left.

local attachment = require("../../../../objects/attachment/attachment")
local drill_object = require("../../../../chips/drilarm/drill")
local lockon = require("../../../../rules/lockon")
local regions = require("@exelib/regions")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- AIAttackVars+0x10: ticks left in the phase.
    timer: number,
}

local ground_drill = {}

-- The phases, on the attack's step.
local BURROW, EMERGE, DRILL, SINK, RETURN = 0x0, 0x4, 0x8, 0xC, 0x10

-- The attack: Break (secondary element 0x10) and its hit parameter.
local BREAK, HIT_PARAM = 0x10, 0x94

-- The drill's time (the original's parameter word 0x1800), and how long
-- the navi stands after it.
local DRILL_TICKS, AFTER = 0x18, 0xA

-- (The sound GroundCross Beast's dash also starts with.)
local SOUND = asset.sound("ground-beast-dash")

-- The dust where it burrows, and the puffs where it sinks and where it
-- comes back up.
local DUST = define.effect { sprite = asset.sprite("ground-drill-effect"), anim = 0, palette = 0 }
local SINK_PUFF = define.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
local RETURN_PUFF = define.effect { sprite = asset.sprite("puff"), anim = 1, palette = 0 }

-- Where the navi waits off the field (X, 16.16).
local AWAY = 0xB4_0000

-- The panel it comes up in front of: one with the other side's navi
-- (`off_8109784`, by side), or else the other side's first panel in the
-- row (`byte_80EF0A4`, by side: require, forbid).
local ENEMY = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
local ENEMY_PANEL = { [0] = { 0x20, 0x00 }, [1] = { 0x00, 0x20 } }

-- The Beast Out lock-on mode it uses (the Beast claw's).
local LOCKON = lockon.beast_claw

-- `sub_8011D74`: 10 damage and 10 more per point of buster Attack (up to
-- 5), Break.
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x0A, 0x0A)
    navi.hit_param = HIT_PARAM
    navi.attack_element = BREAK
    navi.charged = 0
    navi.extra = 0
    navi.attack_lockout = 0
    return ground_drill.action
end

-- `sub_8109746`: the first panel ahead in the navi's row with the other
-- side's navi on it (column 0 if none).
local function enemy_ahead(me: Object): number
    local require = ENEMY[me.alliance]
    if require == nil then
        error(string.format("side %d reads past off_8109784 (sub_8109746)", me.alliance))
    end
    local x = me.panel_x + me.facing
    while true do
        if field.check(x, me.panel_y, require, 0) then
            return x
        end
        x += me.facing
        if x == 0 or x == 7 then
            return 0
        end
        if x < 0 or x > 7 then
            error("sub_8109746 scans past the field's edge forever")
        end
    end
end

-- `object_getClosestPanelMatchingRowFiltered`: from the far column of
-- `side`'s opponent toward `side`'s, the first panel of row `y` that meets
-- `rule` (column 0 if none).
local function closest(side: number, y: number, rule: { number }): number
    local x = 6 - 5 * side
    local step = 2 * side - 1
    while true do
        if field.check(x, y, rule[1], rule[2]) then
            return x
        end
        x += step
        if not field.valid(x, y) then
            return 0
        end
    end
end

-- `sub_80EF004`: once still, burrow: on the 3rd tick a puff of dust and
-- the body leaves the field; after 30, come up by the target.
local function burrow(me: Object, s: State)
    if me.step_init == 0 then
        if me:status("moving") or me:status("sliding") then
            return
        end
        me.step_init = 1
        s.timer = 0x1E
        me:set_animation(0x16)
        me:reserve_panel(me.panel_x, me.panel_y)
        me.collision.region = nil
        me:set_status("moving", true)
    end
    if s.timer == 0x1B then
        local dust = battle.effect(me.pos, DUST)
        if dust then
            dust.timer = 0x1B
            dust.pos = dust.pos - Vec3.new(0, 0x1_0000, 0x1_0000)
        end
        local p = me.pos
        me.pos = Vec3.new(AWAY, p.y, p.z)
    end
    local t = s.timer - 1
    s.timer = t
    if t > 0 then
        return
    end
    local y = me.panel_y
    local x = enemy_ahead(me)
    if x == 0 then
        local rule = ENEMY_PANEL[me.alliance]
        if rule == nil then
            error(string.format("side %d reads past byte_80EF0A4 (sub_80EF004)", me.alliance))
        end
        x = closest(bit32.bxor(me.alliance, 1), y, rule)
    end
    me.panel_x, me.panel_y = me:lockon_panel(x, y, LOCKON)
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    me.collision.region = regions.single
    me.step = EMERGE
    me.step_init = 0
end

-- `sub_80EF0B4`: 10 ticks coming up (standing from the 3rd), then drill
-- (at once: true).
local function emerge(me: Object, s: State): boolean
    if me.step_init == 0 then
        s.timer = 0xA
        me.step_init = 4
    end
    if s.timer == 7 then
        me:set_animation(0)
    end
    local t = s.timer - 1
    s.timer = t
    if t > 0 then
        return false
    end
    me.step = DRILL
    me.step_init = 0
    return true
end

-- `sub_80EF0E2`: the drill arm and the drill's hit, for the drill's time
-- and 10 ticks more. (The parameter word's third byte, 0 here, would make
-- the drill a light one.)
local function drill(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_animation(0xA)
        attachment.attach(me, drill_object.arm, "overlay", { anim = drill_object.ARM_DRILLING })
        local damage = int.u32(bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16)) + me.extra)
        drill_object.spawn(me, me.attack_element, 0x10_0000, damage, { ticks = DRILL_TICKS })
        me:open_counter_window()
        s.timer = DRILL_TICKS + AFTER
        battle.play_sound(SOUND)
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t > 0 then
        return
    end
    me.step = SINK
    me.step_init = 0
end

-- `sub_80EF14A`: lower the arm; 10 ticks later sink in a puff and leave
-- the field.
local function sink(me: Object, s: State)
    if me.step_init == 0 then
        local arm = me.overlay
        if arm then
            -- sub_80B8E70
            arm.anim = 0
            arm.anim_loaded = 0xFF
        end
        s.timer = 0xA
        me.step_init = 8
    end
    local t = s.timer - 1
    s.timer = t
    if t > 0 then
        return
    end
    battle.effect(me.pos + Vec3.new(0, 0, 0x10_0000), SINK_PUFF)
    local fx, fy = field.center(me.future_panel_x, me.future_panel_y)
    battle.effect(Vec3.new(fx, fy, 0x10_0000), RETURN_PUFF)
    me.step = RETURN
    me.step_init = 0
    s.timer = 3
    me.collision.region = nil
    local p = me.pos
    me.pos = Vec3.new(AWAY, p.y, p.z)
    me.related1 = nil
    me.overlay = nil
end

-- `sub_80EF1AE`: 3 ticks, then back on the panel it left, and idle.
local function return_(me: Object, s: State)
    local t = s.timer - 1
    s.timer = t
    if t > 0 then
        return
    end
    me:set_status("using_action", false)
    me:set_status("moving", false)
    me.collision.region = regions.single
    me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
    me:unreserve_panel(me.panel_x, me.panel_y)
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    me:exit_attack()
end

ground_drill.action = define.action {
    id = "groundcross/drill/action",
    state = { timer = "u16" },
    -- `sub_80EEFDC`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == BURROW then
            burrow(me, s)
        elseif step == EMERGE then
            if emerge(me, s) then
                drill(me, s)
            end
        elseif step == DRILL then
            drill(me, s)
        elseif step == SINK then
            sink(me, s)
        elseif step == RETURN then
            return_(me, s)
        else
            error(string.format("GroundCross's charged shot phase %#x reads past its phase table (sub_80EEFDC)", step))
        end
    end,
}

ground_drill.weapon = define.weapon {
    id = "groundcross/drill",
    -- The charge table's row 0x19, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 100, 90, 80, 75, 70, 50, 50, 50 },
    setup = setup,
}

return ground_drill
