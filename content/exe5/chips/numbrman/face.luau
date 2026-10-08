--!strict
-- NumberMan's face (EXE5's actor object #0x5C, 0x080C412E; EXE5's own, NumberMan's alone: its spawner's only caller is
-- his, with its Param0 0 and Param2 1): @exelib/numbrman/face, its sprite 10-06, waiting for every navi to be in.

return require("@exelib/numbrman/face").make {
    id = "numbrman/face",
    sprite = asset.sprite("numberman-face"),
}
