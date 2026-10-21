--!strict
-- SearchMan's party support, Serch-S (EXE5 DS: 0x02004438 offline, 0x020044E8 in a link battle; his controller the DS's
-- effect object 0x94, 0x02226404, his actor its actor object 0x34, 0x021FFD5C; docs/design/exe5ds-map.md §4.7): once a
-- battle, with the light test passed and SearchMan a member who may support (not on the field), the battle dims (the
-- other side can't cut in), Serch-S's telop shows, and SearchMan comes behind the navi on the field and fires five
-- sniper shots at the enemy: offline at the first enemy holding a chip, outside the dimming; in a link battle at the
-- opponent's navi as it starts a chip, which his coming stops. His last shot takes the enemy's chip away (unless a
-- barrier holds it off, or it is frozen).

local chips_serch_s = require("../../chips/serch_s")
local party = require("../../rules/party")
local searchman = require("../searchman").searchman
local shot = require("../../chips/serchman/shot")

-- His damage a shot by the side's level (0x020CC378), its hit flags 0x8A.
local DAMAGE: { number } = { 5, 5, 5, 5, 5, 5, 10, 10, 15, 15 }
local FLAGS = 0x8A
local SERCH_S = chips_serch_s["serch-s"]
-- The panel flag of the other side's navi, by side (`sub_80E7486`'s).
local ENEMY_NAVI = { [0] = 0x00200000, [1] = 0x00400000 }

-- His actor's actions: the lead-in (4 ticks), the appearance (animation 3, 4 ticks), a wait (16), the aim and the
-- shots, a wait (20), the warp out (animation 4, 4 ticks), the lead-out (4 ticks).
local LEAD_IN, APPEAR, READY, SNIPE, STAND, WARP, LEAD_OUT = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14, 0x18
local AIM, SHOOT, AFTER = 0x0, 0x4, 0x8
local SHOTS = 5
-- His scope over the target (the effect table's row 0x68, 20 pixels up) with its sound; his shots (animation 0x11, the
-- shot's sound), each on its fourth tick of ten; the chip taken (row 0x10, the DS's own: the chip destruction spark;
-- the GBA's is the pink flash).
local SCOPE = new.effect { sprite = asset.sprite("search-scope"), anim = 0, palette = 0 }
local TAKEN = new.effect { sprite = asset.sprite("chip-destruction-spark"), anim = 0, palette = 0 }
local SCOPE_SOUND = asset.sound("mark")
local SHOT_SOUND = asset.sound("col-army-2")
local APPEAR_SOUND = asset.sound("appear")

type ActorState = { tx: number, ty: number, shots: number, fired: boolean, controller: Object? }
type ControllerState = { tx: number, ty: number, out: boolean }

local support = {}

local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    return t <= 0
end

local function go(me: Object, action: number)
    me.action, me.phase, me.phase_init = action, 0, 0
end

-- `sub_80E7486`: if the panel shows the other side's navi, the first combatant (both sides' lists) whose collision is
-- on it.
local function combatant_on(x: number, y: number, side: number): Object?
    if not bit32.btest(field.flags(x, y), ENEMY_NAVI[side]) then
        return nil
    end
    for s = 0, 1 do
        for _, o in battle.alive_actors(s) do
            local c = o.collision
            if c.panel_x == x and c.panel_y == y then
                return o
            end
        end
    end
    return nil
end

-- 0x021FFEBD and on: a shot on its fourth tick (the last takes the enemy's chip, unless a barrier holds it off, a
-- bubble but one that is down, or it is frozen).
local function fire(me: Object)
    local s = me.state :: ActorState
    if s.shots == 1 then
        local o = combatant_on(s.tx, s.ty, me.alliance)
        if o then
            local c = o.collision
            local open = c.barrier == "none" or (c.bubble and c.barrier_timer == 0)
            if open and o:status_timer("freeze") == 0 then
                battle.advance_hand(o.alliance)
                o:drop_chip()
                o.chips_held = 0
                local cx, cy = field.center(s.tx, s.ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), TAKEN)
            end
        end
    end
    -- (0x0220005C: its Param1 0x19, plain, and no hit modifier.)
    shot.spawn(me, s.tx, s.ty, false, 0)
end

-- 0x021FFE6D: his aim (the scope; 20 ticks), his shots, his wait.
local function snipe(me: Object)
    local s = me.state :: ActorState
    if me.phase == AIM then
        if me.phase_init == 0 then
            me.anim = 0x10
            local cx, cy = field.center(s.tx, s.ty)
            battle.effect(Vec3.new(cx, cy, 0x14_0000), SCOPE)
            battle.play_sound(SCOPE_SOUND)
            me.timer = 0x14
            me.phase_init = 4
            return
        end
        me.timer -= 1
        if me.timer <= 0 then
            me.phase, me.phase_init = SHOOT, 0
        end
    elseif me.phase == SHOOT then
        local function start_shot()
            me.anim, me.anim_loaded = 0x11, 0x12
            me.timer = 0xA
            battle.play_sound(SHOT_SOUND)
            s.fired = false
            me.phase_init = 4
        end
        if me.phase_init == 0 then
            s.shots = SHOTS
            start_shot()
            return
        end
        if me.timer == 7 and not s.fired then
            s.fired = true
            fire(me)
        end
        me.timer -= 1
        if me.timer > 0 then
            return
        end
        s.shots -= 1
        if s.shots > 0 then
            start_shot()
        else
            me.phase, me.phase_init = AFTER, 0
        end
    elseif me.phase == AFTER then
        if me.phase_init == 0 then
            me.anim = 0
            me.timer = 0x1E
            me.phase_init = 4
            return
        end
        me.timer -= 1
        if me.timer <= 0 then
            go(me, STAND)
        end
    else
        error(string.format("SearchMan's support: phase %#x reads past its table (0x022272F4)", me.phase))
    end
end

-- 0x021FFDC8: his actions, and his sprite.
local function update(me: Object)
    local a = me.action
    if a == LEAD_IN then
        if countdown(me, 4) then
            go(me, APPEAR)
        end
    elseif a == APPEAR then
        if me.phase_init == 0 then
            me.visible = true
            me.anim = 3
            battle.play_sound(APPEAR_SOUND)
        end
        if countdown(me, 4) then
            go(me, READY)
        end
    elseif a == READY then
        if me.phase_init == 0 then
            me.anim = 0
        end
        if countdown(me, 0x10) then
            go(me, SNIPE)
        end
    elseif a == SNIPE then
        snipe(me)
    elseif a == STAND then
        if me.phase_init == 0 then
            me.anim = 0
        end
        if countdown(me, 0x14) then
            go(me, WARP)
        end
    elseif a == WARP then
        if me.phase == 0 then
            me.anim, me.anim_loaded = 4, 0
            me.timer = 4
            me.phase = 4
        end
        me.timer -= 1
        if me.timer <= 0 then
            me.visible = false
            go(me, LEAD_OUT)
        end
    elseif a == LEAD_OUT then
        if countdown(me, 4) then
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("SearchMan's support: action %#x reads past its table (0x022272D8)", a))
    end
    me:update_sprite()
end

support.actor = new.kind {
    id = "searchman/support",
    pool = "actor",
    state = { tx = "u8", ty = "u8", shots = "u8", fired = "bool", controller = "object" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x021FFD6C: on the navi's panel, 10 pixels behind it and 10 up, SearchMan's sprite, his shadow.
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x - me.facing * 0xA_0000, p.y - 0xA_0000, 0)
            local sprite = me.sprite
            sprite:load(searchman.sprite)
            sprite.shadow = "ground"
            me.anim, me.anim_loaded = 0, 0
            sprite:set_animation(0)
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_lifecycle("update")
            update(me)
        elseif l == "update" then
            update(me)
        else
            -- 0x0220001C: his controller's word for him cleared.
            local c = (me.state :: ActorState).controller
            if c then
                (c.state :: ControllerState).out = false
            end
            me:free()
        end
    end,
}

-- 0x02226424: SearchMan comes; once he is gone, the undim.
local function bring(me: Object)
    local s = me.state :: ControllerState
    if me.phase_init == 0 then
        local navi = me.related1 :: Object
        local o = battle.spawn(support.actor)
        if o then
            o.panel_x, o.panel_y = me.panel_x, me.panel_y
            o.alliance, o.flip = navi.alliance, navi.flip
            o.element = me.element
            o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
            o.related1 = navi
            local a = o.state :: ActorState
            a.tx, a.ty, a.controller = s.tx, s.ty, me
            s.out = true
        end
        me.phase_init = 4
    end
    if not s.out then
        me:set_action(0xC)
    end
end

support.controller = new.kind {
    id = "searchman/support-controller",
    pool = "effect",
    state = { tx = "u8", ty = "u8", out = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            dimming.begin(me)
        elseif l == "update" then
            local a = me.action
            if a == 0 then
                dimming.dim_screen(me)
            elseif a == 4 then
                dimming.show_telop(me)
            elseif a == 8 then
                bring(me)
            elseif a == 0xC then
                dimming.undim_screen(me)
            else
                error(string.format("SearchMan's support's controller: action %#x reads past its table (0x0222B3F0)", a))
            end
        else
            dimming.finish(me)
        end
    end,
}

-- 0x020045A0: the target: offline the first enemy (the other side's list of alive actors) holding a chip, in a link
-- battle the opponent's navi.
local function target(side: number): (number?, number?)
    if battle.link() then
        local o = battle.player(1 - side)
        if o then
            return o.panel_x, o.panel_y
        end
        return nil, nil
    end
    for i = 1, 4 do
        local o = battle.alive_actor_slot(1 - side, i)
        if o and o.has_chip then
            return o.panel_x, o.panel_y
        end
    end
    return nil, nil
end

-- The shared gate and the coming (0x02004438, 0x020044E8): true when he comes.
local function call(side: number, link: boolean): boolean
    if party.supports(side).searchman_used or not party.light(side) or battle.dimmed() or battle.link() ~= link then
        return false
    end
    local m = party.member_of(side, searchman)
    if m == nil or not party.may_support(side, searchman) then
        return false
    end
    local navi = battle.player(side)
    if navi == nil or battle.navi(side).navi == searchman then
        return false
    end
    local tx, ty = target(side)
    if tx == nil or ty == nil then
        return false
    end
    party.supports(side).searchman_used = true
    local damage = bit32.bor(DAMAGE[(party.level(side) or party.LEVEL) + 1], bit32.lshift(FLAGS, 16))
    -- (0x0222645C.)
    local o = battle.spawn(support.controller)
    if o then
        o.panel_x, o.panel_y = navi.panel_x, navi.panel_y
        o.element = 0
        o.related1 = navi
        o.alliance, o.flip = navi.alliance, navi.flip
        o:set_damage_word(damage)
        local s = o.state :: ControllerState
        s.tx, s.ty = tx, ty
    end
    dimming.start(side, true, o, navi, { chip = SERCH_S })
    return true
end

-- 0x02004438 (each fight tick, offline).
function support.tick(side: number)
    call(side, false)
end

-- 0x020044E8 (the rules' `opponent_chip`, a link battle's): the opponent's navi starts a chip; SearchMan's coming
-- stops it.
function support.opponent_chip(side: number, _navi: Object): boolean
    return call(side, true)
end

return support
