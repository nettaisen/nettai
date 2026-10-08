--!strict
-- EXE5's RedFrut1-3 (action 0x1A's instant effect 15, 0x080D818C): EXE5's own. A fruit (attack object #0x8D,
-- 0x080D7F2C) drops on a random free panel of the field but the back columns, and hops: 40 ticks shown, 4 ticks going
-- (a puff), then onto a random free panel of the other rows (or of its own, with none), five times; after the sixth
-- going it is gone. A hit breaks it (one HP); the side whose attack, object or body broke it gets its gift on one of
-- its alive navis at random (0x080D82BA): RedFrut1's 300 HP back (AntiRecv turning it) and 50 more mood, RedFrut2's
-- invulnerability for 420 ticks, RedFrut3's Full Synchro (0x080D813C: a player's navi out of the operation battle
-- whose emotion is one of the first four, a mood of 0 left as it is): each chip's own (./init). A hit by both sides at
-- once, or by neither's, gives nothing. Its params are the chip's (+0x10: the gift, which picks the original's routine
-- of 0x080D810C, and its palette; +0x11: its hops).

local collision = require("../../rules/collision")
local panels = require("@exelib/panels")

local fruit = {}

export type State = {
    -- Its chip's gift and palette (Param1 in the original, the gift's number).
    gift: Gift,
    palette: number,
    -- Param2: the hops left (a byte: below 0 it is 0xFF).
    hops: number,
    -- +0x60: the side that broke it (0xFF: none).
    breaker: number,
}

-- 0x080D7F50: sprite 0C-4E without a shadow (`sub_8011504`'s 0x00000C4E), animation 0.
local SPRITE = asset.sprite("red-fruit")
local HP = 1
-- Its collision (0x0E0F0003): a neutral body (row 0x0E) that everything reaches (row 0x0F), hit modifier 0 (the
-- word's third byte; `sub_8011504` reads no fourth).
local HIT_MOD = 0
-- NameID 0xE7: actor record 0xE7 (0x08014C94: version 2, AI index 0x21); EXE5's swallowing and Poltergeist leave it
-- (0x0800D372, 0x080E8CA0).
local IDENTITY = new.identity {
    class = "field_object",
    version = 2,
    ai_index = 0x21,
    scrap = false,
    throwable = false,
}
-- The effects (EXE5's table, 0x080DFC94): row 0x15 (14-0D, animation 3) as it comes, row 0x14 (14-0D, animation 2) as
-- it goes, row 0x0E (14-01) when broken.
local COMES = new.effect { sprite = asset.sprite("puff"), anim = 3, palette = 0 }
local GOES = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
local BROKEN = new.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0 }
local APPEAR_SOUND, BREAK_SOUND = asset.sound("appear"), asset.sound("hit-bomb-1")
local SHOWN_TICKS, PUFF_TICKS = 40, 4
-- Where it neither drops nor hops (0x080D8280): the back columns, reserved while it looks.
local BACK_COLUMNS = { { 1, 1 }, { 1, 2 }, { 1, 3 }, { 6, 1 }, { 6, 2 }, { 6, 3 } }
-- A free panel to land on: solid, with nothing on it (no body, neutral object, blocker or reservation).
local SOLID, OCCUPIED = 0x10, 0x0F880080
-- Its sprite parts not drawn by the hops left (0x080D8314: 6 for more than 5).
local HIDDEN_PARTS = { [0] = 0x1F000000, 0x2F000000, 0x37000000, 0x3B000000, 0x3D000000, 0x3E000000, 0x3F000000 }
-- What broke it, by side (0x080D828C): side 0's attacks, objects and bodies; side 1's.
local SIDE_0, SIDE_1 = 0xAA000000, 0x55000000
local NONE = 0xFF

-- The actions (0x080D8008).
local COME, SHOW, GO = 0, 4, 8

local function state(me: Object): State
    return me.state :: State
end

-- Count the timer down; true when it runs out.
local function count(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

local function next_action(me: Object, action: number)
    me.action = action
    me.phase, me.phase_init = 0, 0
end

-- A random panel of `list` (one RNG2 draw if it has any), unpacked; (0, 0) with none.
local function pick(list: { number }): (number, number)
    if #list == 0 then
        return 0, 0
    end
    local p = list[battle.rng_positive() % #list + 1]
    return panels.x(p), panels.y(p)
end

-- With the back columns reserved by `me` (each one it can).
local function sparing_back_columns(me: Object, look: () -> { number }): (number, number)
    for _, p in BACK_COLUMNS do
        me:reserve_panel(p[1], p[2])
    end
    local x, y = pick(look())
    for _, p in BACK_COLUMNS do
        me:unreserve_panel(p[1], p[2])
    end
    return x, y
end

-- `object_getPanelsIgnoreColumnFiltered` (0x0800B5B0): the free panels of every row but `y`, top row first, left
-- to right.
local function other_rows(y: number): { number }
    local list = {}
    for row = 1, 3 do
        if row ~= y then
            for x = 1, 6 do
                if field.check(x, row, SOLID, OCCUPIED) then
                    list[#list + 1] = panels.pack(x, row)
                end
            end
        end
    end
    return list
end

-- `object_getPanelsInRowFiltered` (0x0800B61C): row `y`'s free panels, left to right.
local function row_of(y: number): { number }
    local list = {}
    for x = 1, 6 do
        if field.check(x, y, SOLID, OCCUPIED) then
            list[#list + 1] = panels.pack(x, y)
        end
    end
    return list
end

-- 0x080D81AC: where it hops (the other rows, else its own).
local function hop_target(me: Object): (number, number)
    return sparing_back_columns(me, function()
        local list = other_rows(me.panel_y)
        if #list == 0 then
            list = row_of(me.panel_y)
        end
        return list
    end)
end

-- 0x080D8014: it comes (a puff 4 pixels up, the sound), 4 ticks.
local function come(me: Object)
    if me.phase_init == 0 then
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x4_0000), COMES)
        me.timer = PUFF_TICKS
        battle.play_sound(APPEAR_SOUND)
        me.phase_init = 4
    end
    if count(me) then
        next_action(me, SHOW)
    end
end

-- 0x080D804E: shown, 40 ticks.
local function show(me: Object)
    if me.phase_init == 0 then
        me.timer = SHOWN_TICKS
        me.phase_init = 4
    end
    if count(me) then
        next_action(me, GO)
    end
end

-- 0x080D8070: it goes (a puff 4 pixels up), 4 ticks; then it hops, or after its last hop it is gone.
local function go(me: Object)
    local s = state(me)
    if me.phase_init == 0 then
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x4_0000), GOES)
        me.timer = PUFF_TICKS
        me.phase_init = 4
    end
    if not count(me) then
        return
    end
    local hops = s.hops - 1
    s.hops = bit32.band(hops, 0xFF)
    if hops < 0 then
        me.collision:set_region(nil)
        obstacle.release_tracking(me)
        obstacle.unregister(me)
        me:set_lifecycle("destroy")
        return
    end
    local x, y = hop_target(me)
    if x ~= 0 then
        me.panel_x, me.panel_y = x, y
        me:set_coordinates_from_panel()
        me:update_collision_panels()
    end
    battle.effect(me.pos, COMES)
    next_action(me, SHOW)
end

-- 0x080D828C: the side whose attack, object or body broke it (alone).
local function breaking_side(me: Object)
    local flags = me.collision.hit_flags
    local by = (if bit32.btest(flags, SIDE_0) then 1 else 0) + (if bit32.btest(flags, SIDE_1) then 2 else 0)
    if by == 1 then
        state(me).breaker = 0
    elseif by == 2 then
        state(me).breaker = 1
    end
end

-- 0x080D7FA0.
local function tick(me: Object)
    local s = state(me)
    local c = me.collision
    c:remove()
    if me:take_damage(0) < 0 then
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), BROKEN)
        battle.play_sound(BREAK_SOUND)
        me.visible = false
        c:set_region(nil)
        breaking_side(me)
        obstacle.release_tracking(me)
        obstacle.unregister(me)
        me:set_lifecycle("destroy")
        return
    end
    if not battle.dimmed() then
        local a = me.action
        if a == COME then
            come(me)
        elseif a == SHOW then
            show(me)
        elseif a == GO then
            go(me)
        else
            error(string.format("RedFrut's fruit action %#x reads past its table", a))
        end
    end
    -- 0x080D82FA: its parts by the hops left.
    local hops = s.hops
    me.sprite.hidden_parts = HIDDEN_PARTS[if hops > 5 then 6 else hops]
    c:present()
end

-- A chip's gift: what it gives the navi its breaker's side gets it on.
export type Gift = { give: (navi: Object) -> () }

function fruit.gift(give: (navi: Object) -> ()): Gift
    return new.record("redfrut-gift", { give = give })
end

-- 0x080D82BA: one of the side's alive navis at random (one RNG2 draw when it has any).
local function navi_of(side: number): Object?
    local list = {}
    for i = 1, 4 do
        local o = battle.alive_actor_slot(side, i)
        if o then
            list[#list + 1] = o
        end
    end
    if #list == 0 then
        return nil
    end
    return list[battle.rng_positive() % #list + 1]
end

-- 0x080D80DE: gone; broken by one side, its gift to a navi of that side first (while the side has none, it waits).
local function leave(me: Object)
    if battle.dimmed() then
        return
    end
    local s = state(me)
    if s.breaker ~= NONE then
        local navi = navi_of(s.breaker)
        if not navi then
            return
        end
        s.gift.give(navi)
    end
    me:destroy()
end

-- 0x080D7F50: on its panel on the ground, then its sprite, collision and name; its chip's palette.
local function init(me: Object)
    local s = state(me)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    -- (sprite_noShadow.)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.neutral_body, collision.everything, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me.identity = IDENTITY
    me.hp, me.max_hp = HP, HP
    sprite.palette = s.palette
    sprite:set_flip(0)
    -- (sub_8002EAC: a drawing flag of the sprite's.)
    me.collision:present()
    s.breaker = NONE
    me:set_lifecycle("update")
    tick(me)
end

fruit.kind = new.kind {
    id = "redfrut/fruit",
    pool = "attack",
    state = { gift = "record:redfrut-gift", palette = "u8", hops = "u8", breaker = "u8" },
    -- 0x080D7F2C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            leave(me)
            if not me.active then
                return
            end
        end
        me:update_sprite()
    end,
}

-- 0x080D8162: the fruit on (x, y), the user's side and flip, running while dimmed, a field object of the user's side.
-- (It spawns at the registers' values, which its init replaces.)
local function spawn(user: Object, x: number, y: number, spec: Fruit)
    local o = battle.spawn(fruit.kind, Vec3.new(y, 0, 0))
    if o then
        local s = o.state :: State
        s.gift, s.palette, s.hops = spec.gift, spec.palette, spec.hops
        o.panel_x, o.panel_y = x, y
        o.alliance, o.flip = user.alliance, user.flip
        o.run_while_dimmed = true
        obstacle.register(o, o.alliance, 1)
    end
end

-- A chip's fruit: its gift, its palette and how many times it hops.
export type Fruit = { gift: Gift, palette: number, hops: number }

-- The chips' `instant` hook (0x080D818C), with the chip's fruit: the fruit on a random free panel but the back
-- columns and the user's own (0x080D821C: `object_getPanelsFiltered`, bottom right first), if there is one.
function fruit.instant(spec: Fruit): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, _spec: InstantChipSpec)
        local ux, uy = user.panel_x, user.panel_y
        user:reserve_panel(ux, uy)
        local x, y = sparing_back_columns(user, function()
            return panels.filtered(SOLID, OCCUPIED)
        end)
        if x ~= 0 then
            spawn(user, x, y, spec)
        end
        user:unreserve_panel(ux, uy)
    end
end

return fruit
