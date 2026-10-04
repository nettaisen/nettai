--!strict
-- The computer navi's buster (./ai's buster run, 0x0802BB7A): the original's weapon routine 0x3E (0x0800FB06),
-- Attack plus one, and the attack it names, action 0x16 (the attack table's row 6, 0x080EC3BC: BN5's own; BN6's
-- action 0x16 is the charged shot): three shots. The navi raises its arm (lib/arm, BN5's 0x080EBAAE) with a muzzle
-- flash in its first related slot (the attachment table's row 13); each shot flies on the second tick from the panel
-- in front (the projectile table's row 3: the plain shot with the charged shot's spark, at 24 pixels, its damage word
-- the attack's damage and counter byte), with the buster's sound, and after five ticks come two of rest; after the
-- third the arm and the flash go.

local arm = require("../../lib/arm")
local attachment = require("../../objects/attachment/attachment")
local buster_parts = require("../../lib/buster")
local projectile = require("../../lib/projectile")
local variants = require("../../objects/projectile/variants")

local buster = {}

export type State = {
    -- AIAttackVars+0x10: ticks into a shot, then ticks of rest left.
    timer: number,
}

-- The shots (AIAttackVars+0x12 counts them down).
local SHOTS = 3
-- A shot's height (16.16).
local SHOT_Z = 0x18_0000
-- The rest after each shot, in ticks.
local REST = 2
local SOUND = asset.sound("bn5:buster-shot")

-- The phases, on the attack's step.
local START, SHOOT, RECOVER = 0x0, 0x4, 0x8

-- 0x080EC3DC: three shots to come; the action in use; the first shot at once (the step's next tick).
local function start(me: Object, _s: State)
    me.attack_count = SHOTS
    me:set_status("using_action", true)
    me.step, me.step_init = SHOOT, 0
end

-- 0x080EC3F4: the arm up (the form overlay restarted, what the overlay slot held let go) and the flash; on the second
-- tick the shot; after five ticks, the rest.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0E)
        me:refresh_form_overlay()
        local held = me.overlay
        if held then
            held:set_lifecycle("destroy")
        end
        arm.raise(me)
        attachment.attach(me, buster_parts.flash, "related")
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(SOUND)
        projectile.fire(me, { variant = variants.bright, damage = me.attack_damage + int.shl(me.hit_param, 16), z = SHOT_Z })
    end
    s.timer += 1
    if s.timer > 4 then
        me.step, me.step_init = RECOVER, 0
    end
end

-- 0x080EC450: two ticks of rest; then the next shot, or, after the third, the arm and the flash go and the attack
-- ends.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = REST
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left
    if left > 0 then
        return
    end
    local shots = me.attack_count - 1
    me.attack_count = shots
    if shots > 0 then
        me.step, me.step_init = SHOOT, 0
        return
    end
    me.related1 = nil
    me.overlay = nil
    me:exit_attack()
end

-- 0x080EC3BC
local function update(me: Object, s: State)
    local step = me.step
    if step == START then
        start(me, s)
    elseif step == SHOOT then
        shoot(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("the computer navi's shots' phase %#x reads past their phase table (0x080EC3D0)", step))
    end
end

buster.action = define.action { id = "bn5:computer-navi/buster", state = { timer = "u16" }, update = update }

-- 0x0800FB06: Attack plus one, no counter byte.
function buster.setup(navi: Object): Action
    navi.attack_damage = battle.navi(navi.alliance).attack + 1
    navi.hit_param = 0
    return buster.action
end

buster.weapon = define.weapon {
    id = "bn5:computer-navi/buster",
    -- The charge table's row 0x3E (0x0801CCD8); Charge 5 reads on into row 0x3F's first entry.
    charge_ticks = { 60, 60, 60, 60, 60, 110 },
    setup = buster.setup,
}

return buster
