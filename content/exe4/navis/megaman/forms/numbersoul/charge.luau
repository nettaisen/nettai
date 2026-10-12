--!strict
-- NumberSoul's charged shot (`numbersoul/charged-shot`): weapon routine 0x11 (0x0800CE08: 10 damage, counter byte
-- 0x94, every other attack variable 0) and the die's throw it starts (action 0x15, 0x080EC164, its alone): the navi
-- throws (animation 10, a counter window, the throw's sound 0x90); on its tenth tick a die (objects/dice: a body the
-- other side can hit) from 24 pixels above it at the panel three columns ahead, its face a draw of the battle's RNG2
-- (0x080EC200's sixteen: a 1, two 2s, three 3s, four 4s, three 5s, three 6s); 22 ticks throwing, then 31 recovering,
-- it exits the attack, handing the lockout on (0x0800C9FC).

local dice = require("../../../../objects/dice")

local charge = {}

local THROW, RECOVER = 0x0, 0x4
local ANIM = 10
-- Its ticks: the throw counts up from 0, the die at 9, to 22; the recovery down from 30 past 0 (`bge`).
local DIE_AT, THROW_END, RECOVER_TICKS = 9, 21, 30
-- The die: three columns ahead, from 24 pixels up, its face by 0x080EC200.
local AHEAD = 3
local DIE_Z = 24 * 0x10000
local FACES = { 1, 2, 2, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 6, 6, 6 }
local SOUND = asset.sound("minibomb-throw")

type State = { timer: number }

-- 0x080EC184.
local function throw(me: Object, s: State)
    if me.step_init == 0 then
        me.anim = ANIM
        battle.play_sound(SOUND)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = 0
        me.step_init = 4
    end
    if s.timer == DIE_AT then
        local face = FACES[bit32.band(battle.rng(), 0xF) + 1]
        local p = me.pos
        local own = dice.own
        dice.made.spawn(me, Vec3.new(p.x, p.y, p.z + DIE_Z), {
            face = face,
            target_x = me.panel_x + AHEAD * me.facing,
            target_y = me.panel_y,
            dimmed = false,
            hp = own.hp,
            status = own.status,
            hit_mod = own.hit_mod,
            damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16)),
            holder = nil,
        })
    end
    s.timer += 1
    if s.timer > THROW_END then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- 0x080EC210.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    s.timer -= 1
    if s.timer < 0 then
        me:exit_attack()
    end
end

charge.action = new.action {
    id = "numbersoul/charged-shot/action",
    state = { timer = "i16" },
    update = function(me: Object, s: State)
        local step = me.step
        if step == THROW then
            throw(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("NumberSoul's throw's phase %#x reads past its phase table", step))
        end
    end,
}

-- 0x0800CE08.
local function setup(navi: Object): Action
    navi.attack_element = 0
    navi.attack_variant = 0
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = 0x94
    navi.attack_damage = 10
    return charge.action
end

charge.weapon = new.weapon {
    id = "numbersoul/charged-shot",
    -- The charge table's row 0x11 (0x080180B8 by AIData +0x0F): 120 ticks at every Charge.
    charge_ticks = { 120, 120, 120, 120, 120, 120 },
    setup = setup,
}

return charge
