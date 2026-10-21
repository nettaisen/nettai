--!strict
-- B-SHOOT (the DS's chip 0x167), the Double Attack of MegaMan and KnightMan (docs/design/exe5ds-map.md §5, §5.2: its
-- director the DS's effect object 0x9E, its actors its T1 0x3F, its balls its T3 0xCE). KnightMan leaps and lands (the
-- cameras shake) and drops six balls in front of himself, one every 21 ticks; MegaMan kicks each with a shot of his
-- buster (his shots hit nothing) into the row above KnightMan's, his own, and the one below in turn, and each flies down
-- its row, a breaking hit on the first thing in it. The chip's record and name are the DS's (written by
-- tools/exe5/gen_content.py, bn6battle-verify, from the DS ROMs); its record's use is the cannon's (action 0x14,
-- subtype 0), which nothing starts: no folder or hand holds it.

local attachment = require("../objects/attachment")
local buster = require("../lib/buster")
local cannon_chips = require("../chips/cannon")
local collision = require("../rules/collision")
local double_attack = require("../lib/double_attack")
local knightman = require("../navis/knightman").knightman
local megaman = require("../navis/megaman").megaman
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

-- The actors' own beside the shared (`DoubleAttackActor`): the row KnightMan's next ball goes to (+0x74: 0 the one above
-- his, 1 his, 2 the one below).
type State = DoubleAttackActor & { next_row: number }

-- A ball's (Param1, its row; the start and the end of its kick, +0x60 to +0x74).
type BallState = { row: number, from: Vec3, to: Vec3 }

local CHARGE_TICKS = 0x3C
-- MegaMan's wait before his first shot, his six shots (21 ticks apart, the first at once), his wait after.
local FIRST_SHOT_WAIT, SHOTS, SHOT_TICKS, AFTER_TICKS = 0x64, 6, 0x14, 0x1E
local ANIM_IDLE, ANIM_JUMP, ANIM_SHOOT = 0, 4, 0xE
local SOUND_SHOT = asset.sound("buster-shot")
-- KnightMan's leap (12 pixels a tick up, half a pixel a tick less each tick), his landing (its sound, a shake), his
-- six balls (21 ticks apart, the first at once).
local LEAP_SPEED, LEAP_FALL = 0xC0000, 0x8000
local SOUND_LAND = asset.sound("land")
local BALLS, BALL_TICKS = 6, 0x14

-- The ball: 10-0E (the pack's `django-rubble`) with a ground shadow, a breaking attack against navis with hit
-- modifier 3 and the breaking spark (`sub_8011504`'s 0x0100100E, 0x06050300); dropped from 190 pixels up, 18 ahead of
-- KnightMan's middle and 10 down, falling an eighth of a pixel a tick faster each tick until 26 pixels up; then kicked
-- (a flash, the effect table's row 0x3C; the hit sound) over 4 ticks to the middle of the panel ahead in its row, at
-- that height; then 8 pixels a tick forward until 150 pixels past the screen's middle.
local BALL_SPRITE = asset.sprite("django-rubble")
local HIT_MOD = 3
local DROP_Z, DROP_X, DROP_Y = 0xBE0000, 0x12, 0xA
local FALL_ACCEL, KICK_Z = 0x2000, 0x1A0000
local KICK = new.effect { sprite = asset.sprite("pink-flash"), anim = 0, palette = 2 }
local SOUND_KICK = asset.sound("damage")
local KICK_TICKS, KICK_PANEL_AT = 4, 2
local FLY_SPEED, FLY_RANGE = 0x80000, 0x96
-- The rows the balls go to by their row (0x020DDEC4).
local ROWS: { number } = { -1, 0, 1 }
-- Its actions.
local FALLS, KICKED, FLIES = 0x0, 0x4, 0x8

-- 0x02033E60: it falls, until 26 pixels up: its region on, kicked.
local function ball_falls(me: Object)
    local v = me.vel
    local vz = v.z - FALL_ACCEL
    me.vel = Vec3.new(v.x, v.y, vz)
    local p = me.pos
    local z = p.z + vz
    if z < KICK_Z then
        me.pos = Vec3.new(p.x, p.y, KICK_Z)
        me.collision:set_region(regions.single)
        me:set_action(KICKED)
        return
    end
    me.pos = Vec3.new(p.x, p.y, z)
end

-- 0x02033F1C: forward 8 pixels a tick; past the screen's reach it goes.
local function ball_flies(me: Object)
    if me.phase_init == 0 then
        me.vel = Vec3.new(me.facing * FLY_SPEED, me.vel.y, me.vel.z)
        me.phase_init = 4
    end
    local x = me.pos.x // 0x10000 + FLY_RANGE
    if x < 0 or x > 2 * FLY_RANGE then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- 0x02033E90: kicked: on its first tick a flash, white, the sound, and its course from where it is to the middle of
-- the panel ahead in its row (on its 2nd tick that panel is its own); 4 ticks (`tdiv`, the DS's division), then it
-- flies.
local function ball_kicked(me: Object)
    me.sprite.white = false
    local s = me.state :: BallState
    if me.phase_init == 0 then
        battle.effect(me.pos, KICK)
        me.sprite.white = true
        battle.play_sound(SOUND_KICK)
        s.from = me.pos
        local row = ROWS[s.row + 1]
        if row == nil then
            error(string.format("B-SHOOT's ball: row %d reads past 0x020DDEC4", s.row))
        end
        local cx, cy = field.center(me.panel_x + me.facing, me.panel_y + row)
        s.to = Vec3.new(cx, cy, s.from.z)
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer == KICK_PANEL_AT then
        me.panel_x, me.panel_y = field.panel_at(s.to.x, s.to.y, me.alliance)
        me:update_collision_panels()
    end
    local t = me.timer + 1
    me.timer = t
    if t >= KICK_TICKS then
        me.action, me.phase, me.phase_init = FLIES, 4, 0
        ball_flies(me)
        return
    end
    local f, to = s.from, s.to
    me.pos = Vec3.new(
        f.x + int.tdiv((to.x - f.x) * t, KICK_TICKS),
        f.y + int.tdiv((to.y - f.y) * t, KICK_TICKS),
        f.z + int.tdiv((to.z - f.z) * t, KICK_TICKS)
    )
end

-- 0x02033E24: the hits against it resolve (one hit, its region off); its action; on a new panel its region on again;
-- its panel lit; its region presented again.
local function ball_tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local x, y = me.panel_x, me.panel_y
    local a = me.action
    if a == FALLS then
        ball_falls(me)
    elseif a == KICKED then
        ball_kicked(me)
    elseif a == FLIES then
        ball_flies(me)
    else
        error(string.format("B-SHOOT's ball: action %#x reads past 0x020DDEB8", a))
    end
    if me.panel_x ~= x or me.panel_y ~= y then
        c:set_region(regions.single)
    end
    field.highlight(me.panel_x, me.panel_y)
    c:present()
end

-- 0x02033DB8: its sprite and collision, shown, 190 pixels over KnightMan's panel 18 pixels ahead and 10 down, its
-- spark, its region off (until it is kicked), at rest; then its first update.
local function ball_init(me: Object)
    local sprite = me.sprite
    sprite:load(BALL_SPRITE)
    sprite.shadow = "ground"
    if not me:create_collision() then
        error("B-SHOOT's ball without a collision slot (0x02033DB8 doesn't check)")
    end
    me:setup_collision(collision.breaking, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.anim, me.anim_loaded = 0, 0xFF
    me.visible = true
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + DROP_X * me.facing * 0x10000, p.y + DROP_Y * 0x10000, DROP_Z)
    local c = me.collision
    c:set_hit_effect(sparks.breaking)
    c:set_region(nil)
    me.vel = Vec3.new(me.vel.x, me.vel.y, 0)
    me:set_lifecycle("update")
    ball_tick(me)
end

local ball = new.kind {
    id = "b-shoot/ball",
    pool = "attack",
    state = { row = "u8", from = "vec3", to = "vec3" },
    -- 0x02033DA4: by state, then the sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            ball_init(me)
        elseif l == "update" then
            ball_tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x02033F60: a ball on KnightMan's panel for `row`, with his damage word, his, running while dimmed.
local function drop_ball(me: Object, row: number)
    local o = battle.spawn(ball, Vec3.new(me.panel_y, 0, 0))
    if not o then
        return
    end
    o.panel_x, o.panel_y = me.panel_x, me.panel_y
    o.element = 0
    o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
    o.alliance, o.flip = me.alliance, me.flip
    o.related1 = me
    o.run_while_dimmed = true
    (o.state :: BallState).row = row
end

local function appear(me: Object)
    double_attack.appear(me, 4, 0x10)
end

-- 0x02033AE8: KnightMan appears (the shared step) and puts on what his identity wears (his ball and chain,
-- `sub_8011420` with Param3 1, a player's identity's: `sub_800F29C`'s actor type 2).
local function knightman_appear(me: Object)
    double_attack.appear(me, 4, 0x10, true)
end

-- 0x0203395C, 0x02033B44: the charge, then the action (KnightMan's in the animation he appeared in).
local function charge(me: Object)
    if double_attack.charge(me, CHARGE_TICKS, nil, (me.state :: State).role == 1) then
        me:set_action(8)
    end
end

-- 0x020339F0: MegaMan waits 100 ticks before his first shot.
local function megaman_wait(me: Object)
    if me.phase_init == 0 then
        me.timer = FIRST_SHOT_WAIT
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me.timer2 = SHOTS
        me.timer = 0
        me.phase, me.phase_init = 4, 0
    end
end

-- 0x02033A18: his shots: his buster arm, its muzzle flash, his overlay restarted (`sub_80C44D2`), 21 ticks apart;
-- after the sixth, the action after.
local function megaman_shoot(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    local s = me.state :: State
    me.anim, me.anim_loaded = ANIM_SHOOT, 0xFF
    battle.play_sound(SOUND_SHOT)
    double_attack.detach(s.held)
    double_attack.megaman_arm(me, "held")
    double_attack.detach(s.held2)
    attachment.attach(me, buster.flash, "held2", { anim = 0, while_dimmed = true })
    local overlay = me.related2
    if overlay then
        overlay.anim_loaded = 0xFF
    end
    me.timer = SHOT_TICKS
    local n = me.timer2 - 1
    me.timer2 = n
    if n <= 0 then
        me:set_action(0xC)
    end
end

-- 0x02033BD4: KnightMan's leap and landing (its sound, the cameras shaking); then his balls, from the row above his.
local function knightman_leap(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_JUMP
        me.vel = Vec3.new(me.vel.x, me.vel.y, LEAP_SPEED)
        me.phase_init = 4
    end
    local v = me.vel
    local vz = v.z - LEAP_FALL
    me.vel = Vec3.new(v.x, v.y, vz)
    local p = me.pos
    local z = p.z + vz
    me.pos = Vec3.new(p.x, p.y, z)
    if vz > 0 or z > 0 then
        return
    end
    battle.play_sound(SOUND_LAND)
    battle.shake_camera(2, 0x14)
    me.anim = ANIM_IDLE
    me.timer2 = BALLS
    me.timer = 0
    (me.state :: State).next_row = 0
    me.phase, me.phase_init = 4, 0
end

-- 0x02033C2C: his balls, 21 ticks apart, each to the next row; after the sixth, the action after.
local function knightman_balls(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    me.timer = BALL_TICKS
    local s = me.state :: State
    drop_ball(me, s.next_row)
    local row = s.next_row + 1
    s.next_row = if row >= #ROWS then 0 else row
    local n = me.timer2 - 1
    me.timer2 = n
    if n <= 0 then
        me:set_action(0xC)
    end
end

local function phases(table_: { [number]: (me: Object) -> () }, who: string): (me: Object) -> ()
    return function(me: Object)
        local f = table_[me.phase]
        if not f then
            error(string.format("B-SHOOT's %s: phase %#x reads past his table", who, me.phase))
        end
        f(me)
    end
end

-- 0x02033A70, 0x02033C68: once the other is in his action after, both attachment slots let go (MegaMan's), 30 ticks,
-- then he leaves.
local function after(me: Object)
    if me.phase_init == 0 then
        if double_attack.partner(me).action ~= 0xC then
            return
        end
        me.anim = ANIM_IDLE
        local s = me.state :: State
        if s.role == 0 then
            s.held, s.held2 = nil, nil
        end
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(0x10)
    end
end

local function leave(me: Object)
    double_attack.leave(me, true)
end

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "special",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 148,
    lockout = 0,
    flags = {},
    action = cannon_chips.cannon.action,
    double_attack = double_attack.make {
        id = "b-shoot",
        damage = { 30, 30, 35, 40, 45, 50, 90, 100, 120, 120 },
        state = { next_row = "u8" },
        roles = {
            -- 0x020DDE54's table: MegaMan, (-16, 10) from the user's spot.
            {
                navi = megaman,
                dx = -16,
                dy = 10,
                actions = {
                    [0x0] = appear,
                    [0x4] = charge,
                    [0x8] = phases({ [0x0] = megaman_wait, [0x4] = megaman_shoot }, "MegaMan"),
                    [0xC] = after,
                    [0x10] = leave,
                },
            },
            -- KnightMan, (-10, -10).
            {
                navi = knightman,
                dx = -10,
                dy = -10,
                actions = {
                    [0x0] = knightman_appear,
                    [0x4] = charge,
                    [0x8] = phases({ [0x0] = knightman_leap, [0x4] = knightman_balls }, "KnightMan"),
                    [0xC] = after,
                    [0x10] = leave,
                },
            },
        },
    },
}

return { ["b-shoot"] = chip }
