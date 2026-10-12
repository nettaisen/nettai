--!strict
-- BugCharg (EXE4 chip 0x136, Red Sun's): a dimming chip whose stand-in raises a charger and fires a heavy shot down
-- its row for each custom screen so far, at most 10. The dimming chips' action 0x0C, its variant 0x4A (0x080220E0's):
-- EXE5's controller and stand-in (@exelib/bugcharg), EXE5's grown from EXE4's. Its controller (effect object #0x7C,
-- 0x080EAB58; its spawner 0x080EABD6 the usual one) sends out EXE4's stand-in (actor object #0x59, 0x080CC72C; spawned
-- by 0x080CC90C): drawn as the user in what its form wears (0x0800B812), standing in animation 10, never hidden from
-- a blind viewer; its charger (an attachment, row 0x2E of 0x080CD524: 0c-43) raised in animation 13 with sound 0xA0;
-- its shots (the flying shot's row 14, 0x080CE690: a heavy hit in the charger's sprite, eight panel centers at 10
-- pixels a tick, sound 0xBF) as many as BattleState +8, the custom screens so far, at most 10 (0x080CC826; under event
-- flag 0x1187, the main subsystem's, never a netbattle's, 5). It takes in no bugs (EXE5's does). Its record is written
-- by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local common_actor = require("@exelib/bugcharg/actor")
local common_controller = require("@exelib/bugcharg/controller")
local flying_shot = require("../objects/flying_shot")
local sparks = require("../lib/sparks")

local CHARGER_SPRITE = asset.sprite("bug-charger")
-- Attachment row 0x2E (0x080CD524): 0c-43 in palette 0, no lift.
local CHARGER = attachment.look { sprite = CHARGER_SPRITE, palette = 0 }
-- The charger's animation as it is raised (0x080CC80C: 1 and twice its navi's number, MegaMan's 0).
local CHARGER_ANIM = 1
-- The flying shot's row 14 (0x080CE690): an attack at navis, hit modifier 3, the heavy spark, the charger's sprite
-- (animation 0, drawn with its sprite), eight panel centers at 10 pixels a tick.
local SHOT = flying_shot.variant {
    hit_mod = 3,
    element = 0,
    spark = sparks.heavy,
    sprite = CHARGER_SPRITE,
    range = 8,
    speed = 0xA_0000,
}
local MOST_SHOTS = 10

local ACTOR = common_actor.make {
    id = "bugcharg/actor",
    -- (0x080CC90C gives it its user's side alone.)
    adopt = function(_me: Object, _user: Object) end,
    -- 0x0800B812: what its side's form wears (MegaMan's soul's overlay), off again as it leaves (0x0800B5DA, while it
    -- wears something).
    wear = function(me: Object, _user: Object)
        me:put_on_form_overlay(battle.navi(me.alliance).form)
    end,
    unwear = function(me: Object, _user: Object)
        if me.related2 then
            me:take_off_form_overlay(battle.navi(me.alliance).form)
        end
    end,
    hides_from_blind = false,
    stand_anim = 10,
    raise_anim = 13,
    raise = function(me: Object)
        attachment.attach(me, CHARGER, "held", CHARGER_ANIM, true)
    end,
    raise_sound = asset.sound("tops"),
    shot_sound = asset.sound("wood-strike"),
    -- 0x080CC826: the custom screens so far, at most 10.
    shots = function(_me: Object): number
        return math.min(battle.turn(), MOST_SHOTS)
    end,
    launch = function(me: Object, pos: Vec3, damage: number): Object?
        return flying_shot.launch(me, pos, SHOT, damage)
    end,
}

local CONTROLLER = common_controller.make { id = "bugcharg/controller", actor = ACTOR.spawn }

local chip: Chip = {
    codes = { "C" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 77,
    damage = 80,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = CONTROLLER.hook,
    megaman = "dark",
    auto_battle = { class = "into_target_row" },
}

return { bugcharg = chip }
