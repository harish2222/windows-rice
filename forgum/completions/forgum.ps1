# forgum PowerShell completion script with argument completer & tooltips
Register-ArgumentCompleter -Native -CommandName 'forgum' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $elements = $commandAst.CommandElements
    $prevWord = if ($elements.Count -ge 2) { $elements[-2].Extent.Text } else { '' }

    $effects = @{
        'walk' = 'Natural bovine walking animation'
        'breathe' = 'Gentle breathing idle animation'
        'float' = 'Drifting levitation animation'
        'fly' = 'Flying creature animation'
        'talk' = 'Conversational speech animation'
        'sway' = 'Swaying pendulum animation'
        'pulse' = 'Pulsing scale animation'
        'glitch' = 'Digital cyberpunk glitch animation'
        'particles' = 'Ambient particle emitter animation'
        'dissolve' = 'Dissolve and materialize animation'
        'animal_natural' = 'Unique per-creature natural kinetic animation'
        'natural' = 'Alias for animal_natural'
    }

    $mountains = @{
        'hills' = 'Rolling verdant hills'
        'peaks' = 'Jagged alpine mountain peaks'
        'volcano' = 'Active smoldering volcano'
        'iceberg' = 'Frozen glacier ridges'
        'skyline' = 'Urban skyscraper silhouette'
        'seamount' = 'Underwater tectonic ridges'
        'plateau' = 'Desert mesa and canyon ridges'
        'crater' = 'Impact rim and astral ridge'
        'gothic' = 'Spire and cathedral silhouette'
        'castle' = 'Fortress ramparts and battlements'
        'garden' = 'Terraced hedge and floral ridge'
        'none' = 'Disable mountain horizon'
    }

    $roads = @{
        'dirt' = 'Countryside dirt trail'
        'cobblestone' = 'Medieval stone pavement'
        'magma' = 'Cracked volcanic basalt'
        'ice' = 'Packed snow and crystalline ice'
        'seabed' = 'Ripple sands and deep coral'
        'sidewalk' = 'Paved urban asphalt'
        'roof' = 'Ceramic roof shingles'
        'grid' = 'Neon vector cyber grid'
        'crypt' = 'Dark catacomb flagstones'
        'savanna' = 'Sun-baked arid trail'
        'mud' = 'Squishy wet soil'
        'tracks' = 'Parallel steel railroad ties'
        'checkerboard' = 'High-contrast dual tiles'
        'none' = 'Disable ground layer'
    }

    $environments = @{
        'pasture' = 'Peaceful open meadows'
        'inferno' = 'Blazing magma chambers'
        'ocean' = 'Deep sea aquatic realm'
        'arctic' = 'Frozen tundra and blizzard'
        'city' = 'Bustling metropolitan skyline'
        'forest' = 'Ancient whispering woodlands'
        'savanna' = 'Sun-drenched golden plains'
        'swamp' = 'Murky bayou and mist'
        'space' = 'Deep cosmos and stellar void'
        'cyber' = 'Neon digital matrix'
        'graveyard' = 'Foggy gothic cemetery'
        'jurassic' = 'Prehistoric primeval wilds'
        'hive' = 'Organic bio-mechanical hive'
        'throne' = 'Imperial gothic sanctuary'
        'none' = 'Disable particle effects'
    }

    $colors = @{
        'default' = 'Creature natural wildlife colors'
        'animal' = 'Alias for default'
        'natural' = 'Authentic God-given natural colors unique to each creature'
        'animal_natural' = 'Alias for natural'
        'rainbow' = 'Refined OKLCH lolcat spectrum'
        'lolcat' = 'Alias for rainbow'
        'solid' = 'Single pure hue'
        'none' = 'Monochrome terminal default'
    }

    $animals = @{
        'apt' = 'Mascot apt'
        'armadillo' = 'Mascot armadillo'
        'atat' = 'Mascot atat'
        'bearface' = 'Mascot bearface'
        'beavis.zen' = 'Mascot beavis.zen'
        'bees' = 'Mascot bees'
        'bill-the-cat' = 'Mascot bill-the-cat'
        'bud-frogs' = 'Mascot bud-frogs'
        'bunny' = 'Mascot bunny'
        'cat' = 'Mascot cat'
        'cat2' = 'Mascot cat2'
        'catfence' = 'Mascot catfence'
        'charizardvice' = 'Mascot charizardvice'
        'charlie' = 'Mascot charlie'
        'claw-arm' = 'Mascot claw-arm'
        'corgi' = 'Mascot corgi'
        'cower' = 'Mascot cower'
        'cowfee' = 'Mascot cowfee'
        'cthulhu-mini' = 'Mascot cthulhu-mini'
        'daemon' = 'Mascot daemon'
        'default' = 'Mascot default'
        'docker-whale' = 'Mascot docker-whale'
        'doge' = 'Mascot doge'
        'dolphin' = 'Mascot dolphin'
        'dragon' = 'Mascot dragon'
        'dragon-and-cow' = 'Mascot dragon-and-cow'
        'duck' = 'Mascot duck'
        'ebi_furai' = 'Mascot ebi_furai'
        'elephant' = 'Mascot elephant'
        'elephant-in-snake' = 'Mascot elephant-in-snake'
        'elephant2' = 'Mascot elephant2'
        'eyes' = 'Mascot eyes'
        'fat-banana' = 'Mascot fat-banana'
        'fat-cow' = 'Mascot fat-cow'
        'fence' = 'Mascot fence'
        'flaming-sheep' = 'Mascot flaming-sheep'
        'fox' = 'Mascot fox'
        'ghost' = 'Mascot ghost'
        'ghostbusters' = 'Mascot ghostbusters'
        'glados' = 'Mascot glados'
        'goat' = 'Mascot goat'
        'goat2' = 'Mascot goat2'
        'golden-eagle' = 'Mascot golden-eagle'
        'happy-whale' = 'Mascot happy-whale'
        'hedgehog' = 'Mascot hedgehog'
        'hellokitty' = 'Mascot hellokitty'
        'hippie' = 'Mascot hippie'
        'hiya' = 'Mascot hiya'
        'hypno' = 'Mascot hypno'
        'jellyfish' = 'Mascot jellyfish'
        'jesus' = 'Mascot jesus'
        'king' = 'Mascot king'
        'kiss' = 'Mascot kiss'
        'kitten' = 'Mascot kitten'
        'kitty' = 'Mascot kitty'
        'knight' = 'Mascot knight'
        'koala' = 'Mascot koala'
        'kosh' = 'Mascot kosh'
        'lamb' = 'Mascot lamb'
        'lamb2' = 'Mascot lamb2'
        'lobster' = 'Mascot lobster'
        'lollerskates' = 'Mascot lollerskates'
        'luke-koala' = 'Mascot luke-koala'
        'mech-and-cow' = 'Mascot mech-and-cow'
        'meow' = 'Mascot meow'
        'hamster' = 'Mascot hamster'
        'minotaur' = 'Mascot minotaur'
        'mona-lisa' = 'Mascot mona-lisa'
        'moofasa' = 'Mascot moofasa'
        'mooghidjirah' = 'Mascot mooghidjirah'
        'moojira' = 'Mascot moojira'
        'moose' = 'Mascot moose'
        'mule' = 'Mascot mule'
        'mutilated' = 'Mascot mutilated'
        'nyan' = 'Mascot nyan'
        'octopus' = 'Mascot octopus'
        'owl' = 'Mascot owl'
        'panther' = 'Mascot panther'
        'pawn' = 'Mascot pawn'
        'periodic-table' = 'Mascot periodic-table'
        'personality-sphere' = 'Mascot personality-sphere'
        'pig' = 'Mascot pig'
        'pterodactyl' = 'Mascot pterodactyl'
        'pufferfish' = 'Mascot pufferfish'
        'queen' = 'Mascot queen'
        'radioactive-kitty' = 'Mascot radioactive-kitty'
        'ram' = 'Mascot ram'
        'ren' = 'Mascot ren'
        'rhino' = 'Mascot rhino'
        'rook' = 'Mascot rook'
        'rooster' = 'Mascot rooster'
        'satanic' = 'Mascot satanic'
        'sauron' = 'Mascot sauron'
        'seahorse' = 'Mascot seahorse'
        'seahorse-big' = 'Mascot seahorse-big'
        'sheep' = 'Mascot sheep'
        'shikato' = 'Mascot shikato'
        'shrug' = 'Mascot shrug'
        'skeleton' = 'Mascot skeleton'
        'sloth' = 'Mascot sloth'
        'small' = 'Mascot small'
        'smiling-octopus' = 'Mascot smiling-octopus'
        'snoopy' = 'Mascot snoopy'
        'snoopyhouse' = 'Mascot snoopyhouse'
        'snoopysleep' = 'Mascot snoopysleep'
        'spidercow' = 'Mascot spidercow'
        'squid' = 'Mascot squid'
        'squirrel' = 'Mascot squirrel'
        'stegosaurus' = 'Mascot stegosaurus'
        'stimpy' = 'Mascot stimpy'
        'supermilker' = 'Mascot supermilker'
        'surgery' = 'Mascot surgery'
        'telebears' = 'Mascot telebears'
        'three-eyes' = 'Mascot three-eyes'
        'tiger' = 'Mascot tiger'
        'tortoise' = 'Mascot tortoise'
        'turkey' = 'Mascot turkey'
        'turtle' = 'Mascot turtle'
        'tux' = 'Mascot tux'
        'tux-big' = 'Mascot tux-big'
        'tweety-bird' = 'Mascot tweety-bird'
        'unipony' = 'Mascot unipony'
        'vader' = 'Mascot vader'
        'viper' = 'Mascot viper'
        'vulpix' = 'Mascot vulpix'
        'walrus' = 'Mascot walrus'
        'weeping-angel' = 'Mascot weeping-angel'
        'whale' = 'Mascot whale'
        'wizard' = 'Mascot wizard'
        'wolf' = 'Mascot wolf'
        'world' = 'Mascot world'
        'yoda' = 'Mascot yoda'
        'random' = 'Mascot random'
    }

    $targetMap = $null
    switch ($prevWord) {
        { $_ -in '--effect', '-e' } { $targetMap = $effects }
        '--mountain' { $targetMap = $mountains }
        '--road' { $targetMap = $roads }
        '--environment' { $targetMap = $environments }
        '--color-mode' { $targetMap = $colors }
        { $_ -in '--cow', '--animal', '-a' } { $targetMap = $animals }
    }

    if ($targetMap) {
        $targetMap.GetEnumerator() | Where-Object { $_.Key -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_.Key, $_.Key, 'ParameterValue', $_.Value)
        }
        return
    }

    $flags = @(
        '--cow', '--animal', '--effect', '--mountain', '--road', '--environment',
        '--color-mode', '--text', '--think', '--background', '--banner',
        '--duration', '--fps', '--thought-interval', '--split-scroll',
        '--reserve-rows', '--reserve-cols', '--split-ratio',
        '--eyes', '--tongue', '--palette'
    )
    $subcommands = @(
        'render', 'think', 'fortune', 'options', 'completions', 'init', 'tui',
        'status', 'doctor', 'checkhealth', 'config', 'logs', 'herd', 'theme',
        'demo', 'showcase', 'say'
    )

    if ($wordToComplete -like '-*') {
        $flags | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterName', $_)
        }
    } else {
        $subcommands | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'Command', $_)
        }
    }
}
