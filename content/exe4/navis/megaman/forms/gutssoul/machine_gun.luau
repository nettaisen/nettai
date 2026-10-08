--!strict
-- GutsSoul's machine gun (`gutssoul/machine-gun`, action 0x10, 0x080EBDC0): what six quick B presses ask for (the
-- form's `rapid_presses`, request 0x20, which EXE4's idle starts as action 0x10 whatever the soul: the role
-- `actions.forced_charged_shot`). The navi turns invulnerable and fires eight shots, each: its arm out (animation 7)
-- with the muzzle flash at it (effect row 5, turned by its side), on the shot's second tick the sound (0xA7) and a
-- projectile from the panel ahead (row 3: 5 damage, counter byte 0x85, 24 high), five ticks a shot and two between
-- them. After the eighth its invulnerability ends, the rapid presses start over, and the attack exits, handing the
-- lockout on (0x0800C9FC).

local collision = require("../../../../rules/collision")
local projectile = require("../../../../lib/projectile")
local shot = require("../../../../objects/projectile")
local sparks = require("../../../../lib/sparks")

local machine_gun = {}

local START, SHOOT, BETWEEN = 0x0, 0x4, 0x8
local SHOTS = 8
local SHOT_TICKS, BETWEEN_TICKS = 4, 2
-- Its arm out, and the muzzle flash at it (effect row 5, 0c-06).
local ANIM = 7
local FLASH = new.effect { sprite = asset.sprite("muzzle-flash"), anim = 0, palette = 0 }
-- Its shot (projectile row 3, 0x080CD2C3): an attack at navis, hit modifier 0, the charged shot's spark.
local SHOT = shot.variant {
    id = "gutssoul/machine-gun/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.charged,
}
local SHOT_DAMAGE = 5 + 0x85 * 0x10000
local SHOT_Z = 24 * 0x10000
local SOUND = asset.sound("machine-gun")
-- Invulnerable all along.
local INVULNERABLE = 0xFFFF

-- (Its shots left are the attack's count, AIAttackVars +0x12.)
type State = { timer: number }

-- 0x080EBDE4.
local function start(me: Object, s: State)
    -- (`object_setInvulnerableTime`, 0x0800B018.)
    me:set_status_timer("invulnerable", INVULNERABLE)
    me:set_status("invulnerable", true)
    me.attack_count = SHOTS
    me:set_status("using_action", true)
    me.step, me.step_init = SHOOT, 0
end

-- 0x080EBE04.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM)
        battle.effect(me.pos, FLASH, me.alliance)
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(SOUND)
        projectile.fire(me, { variant = SHOT, damage = SHOT_DAMAGE, z = SHOT_Z })
    end
    s.timer += 1
    if s.timer > SHOT_TICKS then
        me.step, me.step_init = BETWEEN, 0
    end
end

-- 0x080EBE58.
local function between(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = BETWEEN_TICKS
        me.step_init = 4
    end
    s.timer -= 1
    if s.timer > 0 then
        return
    end
    me.attack_count -= 1
    if me.attack_count > 0 then
        me.step, me.step_init = SHOOT, 0
        return
    end
    -- (0x0800B026.)
    me:clear_invulnerable()
    me.rapid_presses, me.rapid_window = 0, 0
    me:exit_attack()
end

machine_gun.action = new.action {
    id = "gutssoul/machine-gun",
    state = { timer = "i16" },
    update = function(me: Object, s: State)
        local step = me.step
        if step == START then
            start(me, s)
        elseif step == SHOOT then
            shoot(me, s)
        elseif step == BETWEEN then
            between(me, s)
        else
            error(string.format("GutsSoul's machine gun phase %#x reads past its phase table", step))
        end
    end,
}

return machine_gun
