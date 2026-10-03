--!strict
-- LeadRaid's Colonel (BN5's actor object #0x22, 0x080BEAF0; BN5's code, which BN6 keeps unused: `sub_80BE4D8`),
-- brought by LeadRaid's ProtoMan (./protoman) on their user's panel, 10 pixels behind it and 6 up, in his cape. He
-- picks his target as he comes: along his row from his own panel, the first panel holding an enemy navi, else the
-- row's last panel (0x080BED78). He appears (4 ticks), and charges (his glow, ./glow, from his 11th tick) until
-- ProtoMan has picked his own target and 50 ticks have passed; then he draws his sword and strikes an X there (the
-- four diagonals, then the panel itself), and 20 ticks on he lets ProtoMan go on and leaves. Off a panel that isn't
-- solid he waits 10 ticks and leaves.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local glow = require("./glow")
local navi = require("../colonel/navi")
local regions = require("@common/regions")

export type State = {
    -- ProtoMan, who waits for him (+0x54: ProtoMan's +0x18, his `prevent_anim`, which he clears when done).
    protoman: Object?,
    -- ProtoMan has picked his target (+0x60, which ProtoMan sets).
    go: boolean,
    -- His glow while he charges (+0x64).
    glow: Object?,
}

local colonel = {}

local SPRITE = asset.sprite("bn5:colonel")
local SOUND_APPEAR = asset.sound("bn5:appear")
local SOUND_SWING = asset.sound("bn5:spin")
-- His sword (the attachment row 3), its animation 7, animating while dimmed.
local SWORD = attachment.look { sprite = asset.sprite("bn5:sword"), palette = 0 }
local SWORD_ANIM = 7
-- His X (0x080BECC4 rows 1 and 2: the diagonals and the panel, no spark, hitting navis, a slash, hit modifier 3),
-- and its effects (the effect table's rows 0x49 and 0x4A).
local X: { { region: Region, effect: Effect } } = {
    { region = regions.diagonals, effect = define.effect { sprite = asset.sprite("bn5:screen-divide"), anim = 1, palette = 0 } },
    { region = regions.single, effect = define.effect { sprite = asset.sprite("bn5:screen-divide"), anim = 2, palette = 0 } },
}
-- The panels his target holds (0x080BEDE0): an enemy navi's body, by side.
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local SOLID = 0x10
-- His place beside his user's panel (pixels behind and up), his NameID's attach point 0 (sprite 08-07's: 0, 16),
-- where his glow sits.
local BEHIND, UP = 10, 6
local GLOW_X, GLOW_Z = 0, 16
-- When his glow comes, and the least he charges (counting up from 0).
local GLOW_AT, CHARGE = 10, 50
-- When his swing's sound plays and when he strikes (ticks left of 21).
local SWING, STRIKE = 0x11, 0x0C

-- 0x080BED78: his target: along his row from his own panel, the first panel holding an enemy navi; else the row's
-- last panel.
local function find_target(me: Object): (number, number)
    local x, y = me.panel_x, me.panel_y
    local tx, ty = x, y
    while field.valid(x, y) do
        tx, ty = x, y
        if bit32.btest(field.flags(x, y), ENEMY_NAVI[me.alliance]) then
            break
        end
        x += me.facing
    end
    return tx, ty
end

-- Enter the phase with a timer, then count down (from its first tick); true once it has run out.
local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- Action 0 (0x080BEB8C): he appears; 4 ticks; then he charges, or goes off a panel that isn't solid.
local function appear(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
        me.timer2 = 0
    end
    if countdown(me, 4) then
        me:set_action(if field.check(me.panel_x, me.panel_y, SOLID, 0) then 4 else 0x10)
    end
end

-- Action 4 (0x080BEBC6): his glow on his 11th tick; from his 50th, once ProtoMan has picked his target, the glow goes
-- and he strikes.
local function charge(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer == GLOW_AT then
        s.glow = glow.spawn(me, GLOW_X, GLOW_Z)
    end
    local t = int.u16(me.timer + 1)
    me.timer = t
    if t >= CHARGE and s.go then
        glow.stop(s.glow)
        me:set_action(8)
    end
end

-- 0x080BEC5A: his X on his target: each half's hit (running while dimmed) and its effect (running while dimmed).
local function strike_x(me: Object)
    local x, y = me.future_panel_x, me.future_panel_y
    local flip = bit32.bxor(me.alliance, me.flip)
    for _, half in X do
        local box = battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = 0,
            z = 0,
            region = half.region,
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.slash,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 3,
        })
        if box then
            box.run_while_dimmed = true
        end
        local cx, cy = field.center(x, y)
        local e = battle.effect(Vec3.new(cx, cy, 0), half.effect, flip)
        if e then
            e.run_while_dimmed = true
        end
    end
end

-- Action 8, phase 0 (0x080BEC1C): his sword out (animation 5); its sound at 17 left; at 12 the X, the camera shaking
-- (3, 35 ticks) and the screen flashing (30 ticks, going on while dimmed); 21 ticks.
local function strike(me: Object)
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = 5, 0xFF
        attachment.attach(me, SWORD, "related", { anim = SWORD_ANIM, while_dimmed = true })
        me.timer = 0x15
        me.phase_init = 4
    end
    local t = me.timer
    if t == SWING then
        battle.play_sound(SOUND_SWING)
    end
    if me.timer == STRIKE then
        strike_x(me)
        battle.shake_camera(3, 0x23)
        battle.palette_flash(0, 0x1E, true, false)
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- Action 8, phase 4 (0x080BECD0): 20 ticks; then ProtoMan goes on, and he leaves.
local function done(me: Object)
    if countdown(me, 0x14) then
        local protoman = (me.state :: State).protoman
        if protoman then
            protoman.prevent_anim = 0
        end
        me:set_action(0x0C)
    end
end

local ATTACK = { [0] = strike, [4] = done }

local function attack(me: Object)
    local f = ATTACK[me.phase]
    if not f then
        error(string.format("LeadRaid's Colonel: phase %#x reads past his table (0x080BEC14)", me.phase))
    end
    f(me)
end

-- Action 0xC (0x080BECF8): animation 4, his sword away (his first related slot cleared); 4 ticks; his cape off, and
-- he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
        me.related1 = nil
    end
    if countdown(me, 4) then
        me:remove_parts(navi.cape)
        -- (The lifecycle byte alone, as the game stores it.)
        me.lifecycle = "destroy"
    end
end

-- Action 0x10 (0x080BED26): off a panel that isn't solid, standing (animation 0) 10 ticks; then he leaves.
local function off(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 0x0A) then
        me:set_action(0x0C)
    end
end

local ACTIONS = { [0] = appear, [4] = charge, [8] = attack, [0xC] = leave, [0x10] = off }

-- 0x080BEB64.
local function act(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("LeadRaid's Colonel: action %#x reads past his table (0x080BEB78)", me.action))
    end
    action(me)
end

-- 0x080BEB14: his sprite (`sub_801BE2A`'s word 0x01030807: 08-07, animation 3, a ground shadow), his palette
-- (0x0800DA8A: navi 7's count by the chip's level, 0), facing his side's way; beside his user's panel on the
-- ground; his target; his cape (navi 7's parts, stepping while paused); NameID 0x125 (what his glow's place reads);
-- his first update.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 3, 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = 0
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x - me.facing * BEHIND * 0x10000), int.i32(p.y - UP * 0x10000), 0)
    me.future_panel_x, me.future_panel_y = find_target(me)
    me:add_parts(navi.cape, 1)
    me:set_lifecycle("update")
    act(me)
end

colonel.kind = define.kind {
    id = "bn5:leadraid/colonel",
    pool = "actor",
    state = { protoman = "object", go = "bool", glow = "object" },
    -- 0x080BEAF0.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            act(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BED4C: Colonel for `protoman`, on his panel with his element, damage word and user (his user's side and
-- facing); ProtoMan waits for him (his `prevent_anim` set). (His position is the spawner's registers, the row, the
-- element and 0, until his init places him.)
function colonel.spawn(protoman: Object): Object?
    local user = protoman.related1 :: Object
    local o = battle.spawn(colonel.kind, Vec3.new(protoman.panel_y, protoman.element, 0))
    if o then
        o.panel_x, o.panel_y = protoman.panel_x, protoman.panel_y
        o.element = protoman.element
        o.related1 = user
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(bit32.bor(protoman.damage, bit32.lshift(protoman.stamina, 16)))
        local s = o.state :: State
        s.protoman = protoman
        protoman.prevent_anim = 1
    end
    return o
end

-- 0x080BED72: ProtoMan has picked his target: he may strike.
function colonel.go(o: Object?)
    if o then
        (o.state :: State).go = true
    end
end

return colonel
