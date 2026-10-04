--!strict
-- BN5's drills (attack object #0x71) and DrilArm's action (BN5's 0x4F, 0x080EF490; BN6's 0x2E): the shared drill
-- (@exelib/drilarm/drill, BN6's code) but for
--
-- - the action's start: no 10-tick wait outside the operation battle (0x080EF4B8; with it, 10 ticks and effect
--   object 0x83 on the panel ahead, which isn't ported: never in a netbattle), so the arm and drills come at once;
-- - the arm: BN5's attachment row 0x2F (0C-1E, BN6's row 0x20), DarkDril's (subtype 1) with 1 added to its palette;
--   and DarkDril drills for 60 ticks (0x080EF526) where DrilArm's record says 24;
-- - the drills (0x080D41C8, 0x080D4100): they keep no element (0) and the second doesn't take the first's running
--   while dimmed; Param3 is DarkDril's, whose hit carries the bug 0xFA (argument 1), where BN6's makes a light one;
-- - BN5's spark and collision types.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local drill = require("@exelib/drilarm/drill")
local sparks = require("../../lib/sparks")
local operation_battle = require("../../lib/operation_battle")

local bn5_drill = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

local KIND, spawn = drill.make_kind("drill", {
    spark = sparks.breaking,
    first = collision.breaking,
    second = collision.second_drill,
    hits = collision.hits_navis,
    param3 = { bug = 0xFA, bug_arg = 1 },
    keeps_element = false,
})
bn5_drill.kind = KIND

local ARM = attachment.look { sprite = asset.sprite("drill-arm"), palette = 0, attach_point = 0x13 }
local ARM_DRILLING = 1
local SOUND = asset.sound("drilarm")
-- The phases, on the attack's step.
local WAIT, DRILL, DRAW_BACK = 0x0, 0x4, 0x8
-- The drills' height.
local HEIGHT = 0x10_0000
-- DrilArm's ticks (its record's Param2), DarkDril's.
local TICKS, DARK_TICKS = 0x18, 60

-- The action of a drill chip (DarkDril's with `dark`).
function bn5_drill.action(dark: boolean): Action
    -- 0x080EF4EA: the arm and the drills; draw back when the time is up.
    local function grind(me: Object, s: State)
        if me.step_init == 0 then
            me:set_status("using_action", true)
            me:set_animation(0x0A)
            attachment.attach(me, ARM, "overlay", { anim = ARM_DRILLING, palette_add = if dark then 1 else 0 })
            local ticks = if dark then DARK_TICKS else TICKS
            -- The damage word (the hit parameter in its high half), plus the Atk+ bonus.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            spawn(me, me.attack_element, HEIGHT, damage, { ticks = ticks, light = dark })
            me:open_counter_window()
            s.timer = ticks + 0x0A
            battle.play_sound(SOUND)
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t <= 0 then
            me.step = DRAW_BACK
            me.step_init = 0
        end
    end

    -- 0x080EF4B0: outside the operation battle, drill at once; in it, 10 ticks first (and effect 0x83 on the panel
    -- ahead, not ported).
    local function wait(me: Object, s: State)
        if me.step_init == 0 then
            if not operation_battle() then
                me.step = DRILL
                me.step_init = 0
                grind(me, s)
                return
            end
            s.timer = 0x0A
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t > 0 then
            return
        end
        me.step = DRILL
        me.step_init = 0
        grind(me, s)
    end

    -- BN6's `sub_80ED41C` (the same code): the arm goes back; 10 ticks later the navi lets go of the drills.
    local function draw_back(me: Object, s: State)
        if me.step_init == 0 then
            local arm = me.overlay
            if arm then
                arm.anim = 0
                arm.anim_loaded = 0xFF
            end
            s.timer = 0x0A
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t > 0 then
            return
        end
        me:set_status("using_action", false)
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end

    -- 0x080EF490
    local function update(me: Object, s: State)
        local step = me.step
        if step == WAIT then
            wait(me, s)
        elseif step == DRILL then
            grind(me, s)
        elseif step == DRAW_BACK then
            draw_back(me, s)
        else
            error(string.format("DrilArm phase %#x reads past its phase table (0x080EF490)", step))
        end
    end

    return define.action { state = { timer = "u16" }, update = update }
end

return bn5_drill
