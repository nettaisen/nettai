--!strict
-- BugChain (EXE4 chip 0x0D3): a dimming chip that gives its user's NaviCust bugs to the other side: the dimming chips'
-- action 0x0C, its variant 0x1F (0x080220E0's), EXE4's own. Its controller (effect object #0x3F, 0x080E65E0; its
-- spawner 0x080E666C copies the user's side) puts a mark over each side's navi (effect object #0x40, 0x080E6718:
-- sheet 0c-32 for 51 ticks, its sound 0x15D eight ticks in, following its navi) and 61 ticks on copies the bugs
-- (0x080E668E), stat by stat, where the user's is the worse:
--
-- - the move bug (+0x0D) where the user has one that isn't the confusion at the start (0xFF);
-- - the HP drain and the custom drain (+0x0E, +0x0F: the ticks a point, fewer the worse) where the other side has
--   none, or the user's is the faster;
-- - the panel trail (+0x1B) by its rank (0x080E670C: poison 3, broken 2, cracked 1; a patch card's metal or holy
--   trail none) where the user's ranks higher or the other side has none;
-- - the buster's blanks (+0x08) where the user has more;
-- - the charged shot (+0x0A) where the user's is the taunt (routine 0x21, DrkCanon's cost and the weapon bug's);
-- - and NaviStats +0x1A, which nothing writes.
--
-- The other side's weapons take the taunt at their next reload (the engine's, each tick of its intake). (In a battle
-- of type under 0x46, not a netbattle's, the effect ends at once: 0x080E6630.) Its record is written by
-- tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local dimming_chips = require("@exelib/dimming")
-- The taunt (weapon routine 0x21).
local TAUNT: Weapon = require("../navis/megaman/weapons/taunt").weapon

local SPRITE = asset.sprite("bug-chain")
local SOUND = asset.sound("bug-chain")
local MARK_TICKS, SOUND_AT = 50, 42
local WAIT = 60

-- 0x080E6782: 51 ticks, its sound as 42 are left; at its navi's position (and an offset by the navi, 0x080E67F0:
-- MegaMan's none).
local function mark_tick(me: Object)
    if me.phase_init == 0 then
        me.timer = MARK_TICKS
        me.phase_init = 4
    end
    if me.timer == SOUND_AT then
        battle.play_sound(SOUND)
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me:set_lifecycle("destroy")
    end
    local navi = me.related1
    if not navi then
        error("BugChain's mark has no navi (0x080E67AC reads through a null pointer)")
    end
    me.pos = navi.pos
end

local MARK = new.kind {
    id = "bugchain/mark",
    pool = "effect",
    -- 0x080E6718: then the sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080E673C: no shadow, animation 0 from its first frame, stepped once (0x0800285C); its navi's side.
            local navi = me.related1
            if not navi then
                error("BugChain's mark has no navi (0x080E676A reads through a null pointer)")
            end
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "with_sprite"
            me.visible = true
            me.anim, me.anim_loaded = 0, 0
            sprite:set_animation(0)
            sprite:step()
            me.alliance = navi.alliance
            sprite:set_flip(me.alliance)
            me:set_lifecycle("update")
            mark_tick(me)
        elseif l == "update" then
            mark_tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- A panel trail's rank (0x080E670C, by EXE4's panel number).
local function trail_rank(panel: number): number
    if panel == 4 then
        return 3
    elseif panel == 1 then
        return 2
    elseif panel == 3 then
        return 1
    end
    return 0
end

-- A drain's copy (0x080E66AC, 0x080E66BE): the other side's none, or the user's the faster.
local function drain(mine: number, theirs: number): number
    if theirs == 0 or (mine ~= 0 and mine < theirs) then
        return mine
    end
    return theirs
end

-- 0x080E668E: side `from`'s bugs to side `to`.
local function spread(from: number, to: number)
    local a, b = battle.navi(from), battle.navi(to)
    local move = a.move_bug
    if move ~= 0 and move ~= 0xFF then
        b.move_bug = move
    end
    b.hp_drain = drain(a.hp_drain, b.hp_drain)
    b.custom_drain = drain(a.custom_drain, b.custom_drain)
    local trail = a.panel_trail
    if trail ~= 0xFF then
        local rank = trail_rank(trail)
        if rank ~= 0 and (b.panel_trail == 0xFF or rank > trail_rank(b.panel_trail)) then
            b.panel_trail = trail
        end
    end
    if a.buster_blanks > b.buster_blanks then
        b.buster_blanks = a.buster_blanks
    end
    if a.charge_shot_weapon == TAUNT then
        b.charge_shot_weapon = TAUNT
    end
end

-- 0x080E6624: on its first tick a mark over each side's navi (0x0800C456, 0x080E680A); 61 ticks on, the bugs spread.
local function effect(me: Object)
    if me.phase_init == 0 then
        me.timer = WAIT
        for side = 0, 1 do
            local o = battle.spawn(MARK, Vec3.zero)
            if o then
                o.related1 = battle.player(side)
            end
        end
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    spread(me.alliance, bit32.bxor(me.alliance, 1))
    dimming_chips.done(me)
end

local CONTROLLER = new.kind {
    id = "bugchain/controller",
    pool = "effect",
    -- 0x080E65E0.
    update = dimming_chips.phases { name = "BugChain", effect = effect },
}

-- 0x080E666C.
local function hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(CONTROLLER, user, spec, true)
end

local chip: Chip = {
    codes = { "C", "*" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 59,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = hook,
    auto_battle = { class = "when_bugged" },
}

return { bugchain = chip }
