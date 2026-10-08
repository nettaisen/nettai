--!strict
-- ToadSoul's charged shot (`toadsoul/charge`): the original's weapon routine 0x2D (0x0800F9EE) sets 20 damage and 10
-- more per point of buster Attack plus one (0x0800FE5E), its own counter byte, no bonus, the notes' chain (each note
-- 10 ticks, the next 6 ticks in) and its target (0x080BC92C: ToadMan's, the enemy combatant nearest the panel ahead in
-- his cone), and starts action 0x50 (0x080EF5A2): MegaMan blows his horn (the counter window open, ToadMan's melody)
-- and a chain of paralyzing Elec notes (ToadMan's, chips/toadman/note) steps from the panel ahead toward the target.
-- 16 ticks, then 5 of recovery, the horn put away.

local attachment = require("../../../../objects/attachment")
local note = require("../../../../chips/toadman/note")
local toadman = require("../../../../chips/toadman/navi")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: the step's ticks (counted up, then down).
    timer: number,
    -- AIAttackVars +0x2C: what the notes head for.
    target: Object?,
}

local HIT_PARAM = 0x94
local ANIM = 16
-- His horn (the attachment table's row 0x31: sprite 0C-1F, palette 0, at his position).
local HORN = attachment.look { sprite = asset.sprite("toadsoul-horn"), palette = 0 }
local SOUND = asset.sound("toad-melody")
-- The routine's parameters (0x0A0600): no dimming, the next note 6 ticks in, each note 10 ticks.
local CHAIN: { life: number, delay: number, dimmed: boolean } = { life = 10, delay = 6, dimmed = false }
local BLOW_TICKS, RECOVER_TICKS = 15, 5

-- 0x080EF5BC: the horn up, the melody and the first note; 16 ticks.
local function blow(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM)
        me:open_counter_window()
        s.timer = 0
        me:set_status("using_action", true)
        attachment.attach(me, HORN, "overlay", { anim = 0 })
        battle.play_sound(SOUND)
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
        note.spawn(me, me.panel_x + me.facing, me.panel_y, CHAIN, s.target, damage)
        me.step_init = 4
    end
    s.timer += 1
    if s.timer > BLOW_TICKS then
        me.step, me.step_init = 4, 0
    end
end

-- 0x080EF61C: 5 ticks; then the horn put away and back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t <= 0 then
        me:set_status("using_action", false)
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
end

charge.action = new.action {
    id = "toadsoul/charge/action",
    state = { timer = "u16", target = "object" },
    -- 0x080EF5A2
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            blow(me, s)
        elseif step == 4 then
            recover(me, s)
        else
            error(string.format("ToadSoul's charged shot's step %#x reads past its table (0x080EF5B4)", step))
        end
    end,
}

-- 0x0800F9EE
local function setup(navi: Object): Action
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = 20 + 10 * (battle.navi(navi.alliance).attack + 1)
    local s = navi:action_state(charge.action) :: State
    s.target = toadman.find_target(navi)
    return charge.action
end

charge.weapon = new.weapon {
    -- The charge table's row 0x2D (0x0801CC2E); Charge 5 reads on into row 0x2E's first entry.
    charge_ticks = { 140, 135, 130, 125, 120, 60 },
    setup = setup,
}

return charge
