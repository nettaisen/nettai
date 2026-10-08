--!strict
-- RollSoul's charged shot (`rollsoul/charged-shot`): weapon routine 0x22 (0x0800CF20: 30 damage, counter byte 0x94,
-- every other attack variable 0) and the shot it starts (action 0x1B, 0x080EC574): Roll's arrow, the flying shot's
-- row 0 (objects/flying_shot), which destroys a chip of the navi it hits. As it starts the navi raises the bow
-- (animation 10) with a counter window open; on its third tick the arrow sets off 8 pixels ahead of it, a pixel up, 25
-- high; on its fifth the recovery starts: the buster's recovery table's first row by the open panels ahead from the
-- navi's own panel (0x0800B6DC with row 0), and a move may cut it short (EXE4's charged shot's, ../../weapons).

local collision = require("../../../../rules/collision")
local flying_shot = require("../../../../objects/flying_shot")
local projectile = require("../../../../lib/projectile")
local sparks = require("../../../../lib/sparks")

local charge = {}

local SHOOT, RECOVER = 0x0, 0x4
-- The flying shot's row 0 (0x080CE5B0): a chip destruction shot at navis, hit modifier 3, spark 9, the arrow (0c-10,
-- animation 0, drawn with its sprite), eight panel centers at 10 pixels a tick.
local ARROW = flying_shot.variant {
    is = collision.chip_destruction,
    hit_mod = 3,
    element = 0,
    spark = sparks.chip_destruction,
    sprite = asset.sprite("roll-arrow"),
    range = 8,
    speed = 0xA_0000,
}
-- Its height (25 pixels) and its sound as it sets off (0x132).
local ARROW_Z = 25 * 0x10000
local SOUND = asset.sound("roll-arrow")
-- The bow raised.
local ANIM = 10
-- The recovery table's row it counts its recovery by, whatever the navi's Rapid.
local RECOVERY_ROW = 0

type State = { timer: number }

-- 0x080EC594.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = 0
        me.step_init = 4
    end
    if s.timer == 2 then
        local p = me.pos
        local x = int.i32(p.x + me.facing * 8 * 0x10000)
        local y = int.i32(bit32.band(p.y, 0xFFFF0000) - 0x10000)
        flying_shot.launch(me, Vec3.new(x, y, ARROW_Z), ARROW, me.attack_damage + me.hit_param * 0x10000)
        battle.play_sound(SOUND)
    end
    s.timer += 1
    if s.timer >= 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- 0x080EC5F6: the recovery; its end exits the attack, handing the lockout on (0x0800C9FC), and a move may start
-- before it ends.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = projectile.recovery(me, me.panel_x, me.panel_y, RECOVERY_ROW)
        me.step_init = 4
    end
    s.timer -= 1
    if s.timer < 0 then
        me:exit_attack()
        return
    end
    if not me:can_move() then
        return
    end
    local dir = me:held_direction()
    if dir == 0 or me:step_target(dir) == nil then
        return
    end
    me.related1 = nil
    me:exit_attack()
    me:start_move(dir)
end

charge.action = new.action {
    id = "rollsoul/charged-shot/action",
    state = { timer = "i16" },
    update = function(me: Object, s: State)
        local step = me.step
        if step == SHOOT then
            shoot(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("RollSoul's charged shot phase %#x reads past its phase table", step))
        end
    end,
}

-- 0x0800CF20.
local function setup(navi: Object): Action
    navi.attack_damage = 30
    navi.attack_element = 0
    navi.attack_variant = 0
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = 0x94
    return charge.action
end

charge.weapon = new.weapon {
    id = "rollsoul/charged-shot",
    -- The charge table's row 0x22 (0x080180B8 by AIData +0x0F), by Charge 0 to 4; Charge 5 reads on into row 0x23's
    -- first entry.
    charge_ticks = { 60, 60, 60, 60, 60, 30 },
    setup = setup,
}

return charge
