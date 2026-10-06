--!strict
-- GyroSoul's charged shot (`gyrosoul/charge`): the original's weapon routine 9 (0x0800F6AE) sets 15 damage and 5 more
-- per point of buster Attack plus one (0x0800FE5E) and its own counter byte, leaving the rest of the attack as it
-- was, and starts action 0x3C (0x080EE958): MegaMan raises his arm (the attachment row 0x22) and on the sixth tick
-- blows a tornado along the three panels ahead (./tornado); 31 ticks, then 6 of recovery.

local attachment = require("../../../../objects/attachment")
local tornado = require("./tornado")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
}

local HIT_PARAM = 0x8A
-- His arm (the attachment table's row 0x22: sprite 0C-10, palette 0, at his position).
local ARM = attachment.look { sprite = asset.sprite("gyrosoul-arm"), palette = 0 }
local ANIM = 16
-- The steps' ticks, and the tick of the first step's count the tornado comes on.
local BLOW_TICKS, BLOW_AT, RECOVER_TICKS = 30, 25, 5

-- 0x080EE974: the arm up; the tornado on the count's 25; then the recovery.
local function blow(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_animation(ANIM)
        attachment.attach(me, ARM, "overlay", { anim = 0 })
        s.timer = BLOW_TICKS
        me.step_init = 4
    end
    if s.timer == BLOW_AT then
        tornado.spawn(me, me.panel_x + me.facing, me.panel_y, int.u32(me.attack_damage + me.hit_param * 0x10000))
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me.step, me.step_init = 4, 0
    end
end

-- 0x080EE9CC: 6 ticks; then standing again, the arm let go, back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me:set_animation(0)
        me.overlay = nil
        me:exit_attack()
    end
end

charge.action = define.action {
    id = "gyrosoul/charge/action",
    state = { timer = "u16" },
    -- 0x080EE958
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            blow(me, s)
        elseif step == 4 then
            recover(me, s)
        else
            error(string.format("GyroSoul's charged shot's step %#x reads past its table (0x080EE96C)", step))
        end
    end,
}

-- 0x0800F6AE
local function setup(navi: Object): Action
    navi.attack_damage = 15 + 5 * (battle.navi(navi.alliance).attack + 1)
    navi.hit_param = HIT_PARAM
    return charge.action
end

charge.weapon = define.weapon {
    -- The charge table's row 9 (0x0801CAC6); Charge 5 reads on into row 0xA's first entry.
    charge_ticks = { 80, 65, 60, 55, 50, 0 },
    setup = setup,
}

return charge
