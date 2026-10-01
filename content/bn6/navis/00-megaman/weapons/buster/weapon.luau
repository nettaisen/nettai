--!strict
-- MegaMan's buster (`megaman/buster`): the original's weapon routine 0
-- (`sub_8011A26`), which sets up action 0x11 with the buster's damage, and
-- action 0x11 (`sub_80EB436`), the shot (`buster.shot`). The NaviCust's
-- buster bugs may make a shot a blank or a charged shot instead (one RNG
-- draw every shot), and its buster-shot program the shot's kind (another
-- draw). The routine numbers that alias it (0x2E, 0x2F, 0x3E, ...) are
-- this weapon's (its `legacy` marker). See docs/engine/objects-and-player.md
-- §B6.
--
-- Action 0x11 raises the buster; on its second tick the projectile flies
-- from the panel in front (the spread adds shots a row up and down; a
-- throw, `throw_absorbed`'s, throws the obstacle the navi absorbed
-- instead), with a muzzle flash in the navi's first related slot; after
-- five ticks, the recovery, which a move may cut short.

local attachment = require("../../../../objects/attachment/attachment")
local blank_shot = require("../blank-shot/weapon")
local charged_shot = require("../charged-shot/weapon")
local flying_shot = require("../../../../objects/flying-shot/flying_shot")
local lib = require("../../../../lib/buster")
local projectile = require("../../../../lib/projectile")
local variants = require("../../../../objects/projectile/variants")

-- (The recovery after the shot is the navi's `recovery`, AIAttackVars+0x12:
-- a throw doesn't write it, so it waits the last shot's, whatever ran in
-- between.)
export type State = {
    -- Ticks into the shot, then ticks of recovery left (AIAttackVars+0x10).
    timer: number,
    -- What it fires (the original's attack variant 0, 1 or 2): the shot,
    -- the spread (two more shots a row up and down), or the obstacle the
    -- navi absorbed.
    mode: "shot" | "spread" | "throw",
    -- The projectile (the original's first attack parameter, a row of the
    -- projectile's table), which a throw doesn't read.
    projectile: ProjectileVariant?,
}

local buster = {}

-- The phases, on the attack's step.
local SHOOT, RECOVER = 0x0, 0x4

-- The sounds: the shot, and a throw's besides (the junk shot's).
local SHOT_SOUND, THROW_SOUND = asset.sound("buster-6a"), asset.sound("junk-shot")
-- The heights the shot and a thrown obstacle fly at, 16.16.
local SHOT_Z, THROW_Z = 0x18_0000, 0x0C_0000

-- The buster-shot programs from this one on spread on 1 draw in 8, the
-- others on 1 in 2.
local RARE_SPREAD = 0x1E

-- `sub_8013D5E`: what this shot is. Of 16 slots, the first `buster_blanks`
-- are blanks and the next `buster_charged` charged shots; one draw picks a
-- slot.
local function pick(navi: Object): "buster" | "blank" | "charged"
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

-- `sub_8011A26`.
function buster.setup(navi: Object): Action
    local shot = pick(navi)
    if shot == "blank" then
        return blank_shot.setup(navi)
    elseif shot == "charged" then
        return charged_shot.setup(navi)
    end
    navi.attack_damage = navi:buster_damage()
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    -- The buster-shot program (the projectile's row, the navi stats'
    -- NaviCust byte), on a lucky draw.
    local program = battle.navi(navi.alliance).buster_shot
    if program ~= 0 then
        local mask = if program >= RARE_SPREAD then 7 else 1
        if bit32.band(battle.rng_positive(), mask) ~= 0 then
            program = 0
        end
    end
    local s = navi:action_state(buster.shot) :: State
    s.mode = "shot"
    s.projectile = if program == 0 then variants.plain else variants.by_number(program)
    return buster.shot
end

-- The shot as a throw of the obstacle the navi absorbed: what
-- `throw_absorbed`'s setups start instead of a shot.
function buster.throw(navi: Object): Action
    local s = navi:action_state(buster.shot) :: State
    s.mode = "throw"
    return buster.shot
end

-- The attack's damage word: its damage | its hit parameter << 16
-- (AIAttackVars+0x08 as a word).
local function damage_word(me: Object): number
    return bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
end

-- `sub_80EB450`'s throw: the last obstacle absorbed (what the throw's
-- setup took from the absorbed list into the attack: the original's marker
-- word) flies from the panel in front (`sub_80C6248`).
local function throw(me: Object)
    local x, y = field.center(me.panel_x + me.facing, me.panel_y)
    flying_shot.launch(me, Vec3.new(x, y, THROW_Z), {
        variant = flying_shot.variants.thrown,
        damage = damage_word(me),
        obstacle = me.thrown_look,
        obstacle_anim = me.thrown_anim,
    })
end

-- `sub_80EB450`: raise the buster; on the second tick the shot, or the
-- spread's three, or the throw, and the attachment in the first related
-- slot; after five ticks, the recovery. (The attachment in the first
-- related slot is the muzzle flash, or after a throw a second buster arm:
-- the attachment kind is what the throw left in the register. The game
-- also keeps the spread's side shots' rows in the attack's second
-- parameter, which nothing reads back.)
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0E)
        lib.raise_arm(me)
        me:set_status("using_action", true)
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(SHOT_SOUND)
        local flash = lib.flash
        if s.mode == "throw" then
            battle.play_sound(THROW_SOUND)
            throw(me)
            flash = lib.arm
        else
            -- (Without a setup's projectile, the table's first row.)
            local variant, damage = s.projectile or variants.plain, damage_word(me)
            me.recovery = projectile.fire(me, { variant = variant, damage = damage, z = SHOT_Z })
            if s.mode == "spread" then
                projectile.fire(me, { variant = variant, damage = damage, z = SHOT_Z, row = 1 })
                projectile.fire(me, { variant = variant, damage = damage, z = SHOT_Z, row = -1 })
            end
        end
        attachment.attach(me, flash, "related")
    end
    s.timer += 1
    if s.timer > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80EB502`: the recovery; a move may cut it short.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = me.recovery
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
    lib.move_cancel(me)
end

-- `sub_80EB436`.
local function update(me: Object, s: State)
    local step = me.step
    if step == SHOOT then
        shoot(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("buster shot phase %#x reads past its phase table", step))
    end
end

-- The shot; a thrown obstacle (`throw_absorbed`'s) is one too.
buster.shot = define.action {
    id = "megaman/buster/shot",
    state = { timer = "u16", mode = { "shot", "spread", "throw" }, projectile = "record:projectile-variant" },
    update = update,
}

buster.weapon = define.weapon {
    id = "megaman/buster",
    name = "Buster",
    -- The buster isn't charged (the charge table's row 0).
    charge_ticks = { 0, 0, 0, 0, 0 },
    setup = buster.setup,
    -- (Transitional: the routine numbers the pack's forms, the navis'
    -- rows and the ruleset name it by, `off_80117D4`'s entries that are
    -- `sub_8011A26`.)
    legacy = { routines = { 0x00, 0x2E, 0x2F, 0x3E, 0x3F, 0x4D, 0x4E, 0x4F, 0x50, 0x51, 0x6F, 0x70, 0x77, 0x79, 0x7B, 0x7E, 0x82 } },
}

return buster
