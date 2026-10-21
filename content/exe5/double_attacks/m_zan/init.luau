--!strict
-- M-ZAN (the DS's chip 0x161; Magnet Zan), the Double Attack of ProtoMan and MagnetMan (docs/design/exe5ds-map.md §5,
-- §5.2: its director the DS's effect object 0x98, its actors its T1 0x36). MagnetMan raises his magnet arm and holds
-- every enemy body from his column forward in place (MagnetSoul's immobilizer, the DS's with a 120-tick linger and its
-- other sound); ProtoMan charges for 120 ticks, then goes down the columns ahead and, for each enemy whose panel before
-- it is free (or the user's), steps there and slashes the column in front of him (a hit on the enemy's panel and the
-- two beside it), one enemy after another; MagnetMan leaves once ProtoMan is done. The chip's record and name are the
-- DS's (written by tools/exe5/gen_content.py, bn6battle-verify, from the DS ROMs); its record's use is the cannon's
-- (action 0x14, subtype 0), which nothing starts: no folder or hand holds it.

local attachment = require("../objects/attachment")
local cannon_chips = require("../chips/cannon")
local collision = require("../rules/collision")
local double_attack = require("../lib/double_attack")
local immobilizer = require("@exelib/instant/immobilizer")
local magnetman = require("../navis/magnetman").magnetman
local protoman = require("../navis/protoman").protoman
local regions = require("@exelib/regions")
local status = require("../rules/status")

-- The actors' own beside the shared (`DoubleAttackActor`): the other actor (+0x60, which the DS's director links), and
-- ProtoMan's search starting at the top of its column (+0x70; after a slash it goes on below the last enemy).
type State = DoubleAttackActor & { partner: Object?, from_top: boolean }

-- ProtoMan's charge (120 ticks, the charged glow 80 ticks before its end, for those 80), his steps (3 ticks each), his
-- swing (20 ticks, its hit on the 10th), his wait after (10 ticks).
local CHARGE_TICKS, CHARGED_AT = 0x78, 0x50
local CHARGING = new.effect { sprite = asset.sprite("charge-glow"), anim = 0, palette = 0 }
local CHARGED = new.effect { sprite = asset.sprite("charge-glow"), anim = 2, palette = 0 }
local CHARGING_LASTS = 0x28
local STEP_TICKS, SWING_TICKS, SWING_HITS_AT, AFTER_TICKS = 3, 0x14, 0xA, 0xA
local ANIM_IDLE, ANIM_STEP, ANIM_SWING, ANIM_MAGNET = 0, 4, 5, 0x10
local SOUND_CHARGING = asset.sound("buster-charge")
local SOUND_CHARGED = asset.sound("buster-charged")
local SOUND_SWING = asset.sound("sword-swing")
-- His sword (the attachment table's row 3, as his navi chip's) in its animation 1; the slash over the column (the
-- effect table's row 0x33), 16 pixels up.
local SWORD = attachment.look { sprite = asset.sprite("sword"), palette = 0 }
local SWORD_ANIM = 1
local SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }
local SWING_Z = 0x100000
local HIT_MOD = 1
-- What ProtoMan looks for (0x02138FB8, by his side): an enemy body; and the panel before it: solid, with no object or
-- reservation on it (flag 0x10, none of 0x0F800000).
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }
local STANDABLE, TAKEN = 0x10, 0x0F800000

-- MagnetMan's magnet arm (the DS's new attachment row 0x3A: 10-3C), animating while dimmed; how long he holds it.
local MAGNET_ARM = attachment.look { sprite = asset.sprite("magnet-arm"), palette = 0 }
local MAGNET_TICKS = 0x78
-- The DS's immobilizer (T3 0x3F, the GBA's code but that its linger is its Param1, 0x78, and its Param2 1 plays 0xFD):
-- MagnetSoul's mark, its guard-breaking hit against navis that immobilizes (status 0x42).
local IMMOBILIZER = immobilizer.make_kind("m-zan/immobilizer", {
    sprite = asset.sprite("immobilized"),
    anim = 1,
    lasts = 0x78,
    sound = asset.sound("bass-anly"),
    status = status.immobilize_120,
    target = collision.hits_navis,
    self_type = collision.guard_breaking,
})

-- The other actor of this Double Attack (the DS's director writes each one's +0x60 as it brings them).
local function partner_of(me: Object): Object
    local s = me.state :: State
    if s.partner == nil then
        for _, o in battle.objects_of(me.kind) do
            if o ~= me and (o.state :: State).director == s.director then
                s.partner = o
            end
        end
    end
    local p = s.partner
    if p == nil then
        error("M-ZAN's actor has no partner (the DS reads through a null pointer)")
    end
    return p
end

-- 0x020A5110, 0x020A53E4: each appears (the shared step), learning its partner as it first runs.
local function appear(me: Object)
    partner_of(me)
    double_attack.appear(me, 4, 0x10)
end

-- 0x020A5158: ProtoMan's charge, 120 ticks; then his search from the column in front, its top row.
local function charge(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_IDLE
        double_attack.at_hand(me, CHARGING, CHARGING_LASTS)
        battle.play_sound(SOUND_CHARGING)
        me.timer = CHARGE_TICKS
        me.phase_init = 4
    end
    if me.timer == CHARGED_AT then
        double_attack.at_hand(me, CHARGED, CHARGED_AT)
        battle.play_sound(SOUND_CHARGED)
    end
    if not double_attack.ran_out(me) then
        return
    end
    me.future_panel_x, me.future_panel_y = me.panel_x + me.facing, 1
    (me.state :: State).from_top = true
    me:set_action(8)
end

-- 0x020A5204, each tick: down the columns from the one his search reached (his future panel), each from its top row
-- (or on below the last enemy), the first enemy body whose panel before it is the user's or solid and free: there his
-- step, else his action after.
local function search(me: Object)
    local s = me.state :: State
    local front = me.facing
    local user = me.related1 :: Object
    local x, y = me.future_panel_x, me.future_panel_y
    if not field.valid(x, y) then
        me:set_action(0xC)
        return
    end
    local enemy = ENEMY_BODY[me.alliance]
    -- (The DS's labels: its column's top, a check, the next row.)
    local at = if s.from_top then "top" else "next"
    while true do
        if at == "top" then
            y = 1
            at = "check"
        end
        if at == "check" then
            if field.check(x, y, enemy, 0) then
                local bx = x - front
                if (bx == user.panel_x and y == user.panel_y) or field.check(bx, y, STANDABLE, TAKEN) then
                    me.future_panel_x, me.future_panel_y = x, y
                    s.from_top = false
                    me.phase, me.phase_init = 4, 0
                    return
                end
            end
        end
        y += 1
        if y <= 3 then
            at = "check"
        else
            x += front
            if x < 1 or x > 6 then
                me:set_action(0xC)
                return
            end
            at = "top"
        end
    end
end

-- 0x020A5294: his sword let go, his step's animation, 3 ticks.
local function step_out(me: Object)
    if me.phase_init == 0 then
        double_attack.detach((me.state :: State).held)
        me.anim = ANIM_STEP
        me.timer = STEP_TICKS
        me.phase_init = 4
        return
    end
    if double_attack.ran_out(me) then
        me.phase, me.phase_init = 8, 0
    end
end

-- 0x020A52C0: onto the panel before the enemy, 3 ticks.
local function step_in(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_STEP
        me.timer = STEP_TICKS
        me.panel_x, me.panel_y = me.future_panel_x - me.facing, me.future_panel_y
        me:set_coordinates_from_panel()
        me.phase_init = 4
        return
    end
    if double_attack.ran_out(me) then
        me.phase, me.phase_init = 0xC, 0
    end
end

-- 0x020A52F8: his swing (its sword, its sound), on its 10th tick a hit over the column ahead (the enemy's panel and the
-- two beside it, 16 pixels up, no spark, as a slash, hit modifier 1) and the slash; then the search goes on.
local function swing(me: Object)
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = ANIM_SWING, ANIM_SWING + 1
        attachment.attach(me, SWORD, "held", { anim = SWORD_ANIM, while_dimmed = true })
        battle.play_sound(SOUND_SWING)
        me.timer = SWING_TICKS
        me.phase_init = 4
        return
    end
    if me.timer == SWING_HITS_AT then
        local x, y = me.panel_x + me.facing, me.panel_y
        double_attack.hit(me, {
            panel_x = x,
            panel_y = y,
            element = 0,
            z = SWING_Z,
            region = regions.wide,
            target = collision.hits_navis,
            self_type = collision.slash,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD,
        })
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, SWING_Z), SLASH, bit32.bxor(me.alliance, me.flip))
    end
    if double_attack.ran_out(me) then
        me.phase, me.phase_init = 0, 0
    end
end

-- 0x020A51F4 (its phases, 0x02138FA8): the search, the step out, the step in, the swing.
local function slash(me: Object)
    local p = me.phase
    if p == 0 then
        search(me)
    elseif p == 4 then
        step_out(me)
    elseif p == 8 then
        step_in(me)
    elseif p == 0xC then
        swing(me)
    else
        error(string.format("M-ZAN's ProtoMan: phase %#x reads past 0x02138FA8", p))
    end
end

-- 0x020A537C: his sword ends, 10 ticks, then he leaves.
local function protoman_after(me: Object)
    if me.phase_init == 0 then
        double_attack.detach((me.state :: State).held)
        me.anim = ANIM_IDLE
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(0x10)
    end
end

-- 0x020A542C: MagnetMan raises his magnet arm for 120 ticks and puts the immobilizer (running while dimmed) on every
-- enemy body from his column forward, row by row (0x02007BA8).
local function magnet(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_MAGNET
        me.timer = MAGNET_TICKS
        attachment.attach(me, MAGNET_ARM, "held", { anim = 0, while_dimmed = true })
        local enemy = ENEMY_BODY[me.alliance]
        for y = 1, 3 do
            local x = me.panel_x
            while field.valid(x, y) do
                if bit32.btest(field.flags(x, y), enemy) then
                    local o = immobilizer.spawn(IMMOBILIZER, me, x, y)
                    if o then
                        o.run_while_dimmed = true
                    end
                end
                x += me.facing
            end
        end
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(8)
    end
end

-- 0x020A546C: he waits for ProtoMan's action after.
local function wait_for_protoman(me: Object)
    if partner_of(me).action == 0xC then
        me:set_action(0xC)
    end
end

-- 0x020A5480: his arm let go (its slot cleared), 10 ticks, then he leaves.
local function magnetman_after(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = ANIM_IDLE
        s.held, s.held2 = nil, nil
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(0x10)
    end
end

local function leave(me: Object)
    double_attack.leave(me, false)
end

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "special",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 158,
    lockout = 0,
    flags = {},
    action = cannon_chips.cannon.action,
    double_attack = double_attack.make {
        id = "m-zan",
        damage = { 50, 50, 70, 80, 90, 100, 180, 200, 240, 240 },
        state = { partner = "object", from_top = "bool" },
        roles = {
            -- 0x020A5100's table: ProtoMan, (10, 10) from the user's spot.
            {
                navi = protoman,
                dx = 10,
                dy = 10,
                actions = { [0x0] = appear, [0x4] = charge, [0x8] = slash, [0xC] = protoman_after, [0x10] = leave },
            },
            -- 0x020A53D4's: MagnetMan, (-10, -10).
            {
                navi = magnetman,
                dx = -10,
                dy = -10,
                actions = {
                    [0x0] = appear,
                    [0x4] = magnet,
                    [0x8] = wait_for_protoman,
                    [0xC] = magnetman_after,
                    [0x10] = leave,
                },
            },
        },
    },
}

return { ["m-zan"] = chip }
