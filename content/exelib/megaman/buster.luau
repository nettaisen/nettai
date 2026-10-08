--!strict
-- MegaMan's buster shot, as EXE5 and EXE6 share it (EXE6's action 0x11, `sub_80EB436`; EXE5's 0x11, the same code but
-- for the muzzle flash's attachment row and the absorbed-obstacle throw, which EXE5 hasn't), of a game's
-- `MegaManBusterLook` (`buster.shot_action`): the navi raises the buster; on its second tick the projectile flies
-- from the panel in front (the spread adds shots a row up and down; a throw throws the obstacle the navi absorbed
-- instead), with a muzzle flash in the navi's first related slot; after five ticks, the recovery, which a move may
-- cut short. And the NaviCust's buster bugs' pick (`sub_8013D5E`). See docs/engine/objects-and-player.md §B6.

local attachment = require("../attachment")

-- (The recovery after the shot is the navi's `attack_count`, AIAttackVars+0x12:
-- a throw doesn't write it, so it waits the last shot's, whatever ran in
-- between.)
export type State = {
    -- Ticks into the shot, then ticks of recovery left (AIAttackVars+0x10).
    timer: number,
    -- What it fires, as its setup states (the original's attack variant 0,
    -- 1 or 2): the shot; the spread, two more shots a row up and down
    -- (`spread`); or the setup's throw (`throw`: EXE6's of the obstacle the
    -- navi absorbed).
    spread: boolean,
    throw: MegaManBusterThrow?,
    -- The projectile (the original's first attack parameter, a row of the
    -- projectile's table), which a throw doesn't read.
    projectile: ProjectileVariant?,
}

local buster = {}

-- A setup's throw: what flies instead of the shot (its sound, then it).
function buster.throw(run: (me: Object) -> ()): MegaManBusterThrow
    return new.record("megaman-buster-throw", { run = run })
end

-- The phases, on the attack's step.
local SHOOT, RECOVER = 0x0, 0x4

-- The heights the shot flies at, 16.16.
local SHOT_Z = 0x18_0000

-- `sub_8013D5E`: what this shot is. Of 16 slots, the first `buster_blanks`
-- are blanks and the next `buster_charged` charged shots; one draw picks a
-- slot. (EXE5's buster picks the same way.)
function buster.pick(navi: Object): "buster" | "blank" | "charged"
    local stats = battle.navi(navi.alliance)
    local blanks, charged = stats.buster_blanks, stats.buster_charged
    if blanks + charged > 16 then
        error(
            string.format(
                "%d blank and %d charged slots overflow sub_8013D5E's 16-slot table (over its stack frame)",
                blanks,
                charged
            )
        )
    end
    local i = bit32.band(battle.rng(), 0xF)
    if i < blanks then
        return "blank"
    elseif i < blanks + charged then
        return "charged"
    end
    return "buster"
end

-- The attack's damage word: its damage | its hit parameter << 16
-- (AIAttackVars+0x08 as a word).
local function damage_word(me: Object): number
    return bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
end

-- `sub_80EB450`: raise the buster; on the second tick the shot, or the
-- spread's three, or the throw, and the attachment in the first related
-- slot; after five ticks, the recovery. (The attachment in the first
-- related slot is the muzzle flash, or after a throw a second buster arm:
-- the attachment kind is what the throw left in the register. The game
-- also keeps the spread's side shots' rows in the attack's second
-- parameter, which nothing reads back.)
local function shoot(me: Object, s: State, look: MegaManBusterLook)
    if me.step_init == 0 then
        me:set_animation(0x0E)
        look.buster.raise_arm(me)
        me:set_status("using_action", true)
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(look.shot_sound)
        local flash = look.buster.flash
        local throw = s.throw
        if throw then
            throw.run(me)
            flash = look.buster.arm
        else
            -- (Without a setup's projectile, the table's first row.)
            local variant, damage = s.projectile or look.plain, damage_word(me)
            local projectile = look.projectiles
            me.attack_count = projectile.fire(me, { variant = variant, damage = damage, z = SHOT_Z })
            if s.spread then
                projectile.fire(me, { variant = variant, damage = damage, z = SHOT_Z, row = 1 })
                projectile.fire(me, { variant = variant, damage = damage, z = SHOT_Z, row = -1 })
                -- (EXE5's spread has its own flash: 0x080EBA30.)
                flash = look.spread_flash or flash
            end
        end
        attachment.attach(look.attachment, me, flash, "related")
    end
    s.timer += 1
    if s.timer > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80EB502`: the recovery; a move may cut it short.
local function recover(me: Object, s: State, look: MegaManBusterLook)
    if me.step_init == 0 then
        s.timer = me.attack_count
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left
    if left < 0 then
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
        return
    end
    look.buster.move_cancel(me)
end

-- `sub_80EB436`.
local function update(me: Object, s: State, look: MegaManBusterLook)
    local step = me.step
    if step == SHOOT then
        shoot(me, s, look)
    elseif step == RECOVER then
        recover(me, s, look)
    else
        error(string.format("buster shot phase %#x reads past its phase table", step))
    end
end

-- A game's shot, keyed `look.id`.
function buster.shot_action(look: MegaManBusterLook): Action
    return new.action {
        id = look.id,
        state = {
            timer = "u16",
            spread = "bool",
            throw = "record:megaman-buster-throw",
            projectile = "record:projectile-variant",
        },
        update = function(me: Object, s: State)
            update(me, s, look)
        end,
    }
end

-- The damage word a throw carries (the attack's damage and hit parameter).
buster.damage_word = damage_word

return buster
