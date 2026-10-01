--!strict
-- DrilArm (the original's action 0x2E, `sub_80ED374`): ten ticks in, the
-- navi's arm becomes a drill (an attachment) and two drills
-- (objects/drill) grind the two panels ahead for 24 ticks plus 10; the
-- navi then draws the drill back over 10 ticks, which ends them.

local attachment = require("../../objects/attachment/attachment")
local drill = require("../../objects/drill/drill")
local lockon = require("../../rules/lockon")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- The phases, on the attack's step.
local WAIT, DRILL, DRAW_BACK = 0x0, 0x4, 0x8
-- The drills' height.
local HEIGHT = 0x10_0000
-- How long the drills hit (the record's second parameter).
local DRILL_TICKS = 0x18
local SOUND = asset.sound("drilarm")

-- `sub_80ED3B6`: the arm and the drills; draw back when the time is up.
local function grind(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_animation(0x0A)
        attachment.attach(me, drill.arm, "overlay", { anim = drill.ARM_DRILLING })
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        drill.spawn(me, me.attack_element, HEIGHT, damage, { ticks = DRILL_TICKS })
        me:open_counter_window()
        s.timer = DRILL_TICKS + 0x0A
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

-- `sub_80ED394`: wait 10 ticks, then start drilling at once.
local function wait(me: Object, s: State)
    if me.step_init == 0 then
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

-- `sub_80ED41C`: the arm goes back (its animation 0); 10 ticks later the
-- navi lets go of the drills and is back to idle.
local function draw_back(me: Object, s: State)
    if me.step_init == 0 then
        -- sub_80B8E70
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

-- `sub_80ED374`.
local function update(me: Object, s: State)
    local step = me.step
    if step == WAIT then
        wait(me, s)
    elseif step == DRILL then
        grind(me, s)
    elseif step == DRAW_BACK then
        draw_back(me, s)
    else
        error(string.format("DrilArm phase %#x reads past its phase table", step))
    end
end

return define.chip {
    id = "drilarm",
    name = "DrilArm",
    description = "Knocks\nenmy 2sq\naway",
    codes = { "G", "M", "W" },
    element = "null", family = "break", class = "standard",
    rarity = 2, mb = 32, damage = 70, hit_param = 10, lockout = 0,
    flags = { "has_damage", "library" },
    library = { number = 52, index = 52, sort = 131 },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.thunder },
    action = define.action { state = { timer = "u16" }, update = update },
}
