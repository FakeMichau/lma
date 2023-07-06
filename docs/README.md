# Docs

## First steps

Before anything - launch the application, first time setup will get you through things you need to setup. Information at the bottom with help with the navigation.

After logging in you see the main menu with no shows and episodes. To add a show press N. 

You will be asked for a folder that contains all of the episodes for a given show. You can copy and paste a path, drag and drop a folder or manually type it in. Press ENTER to confirm.

The program will try to guess the title of your show. Correct it if it's wildly wrong to allow next step to work.

After confirming your title, if you have set up an external service (like MyAnimeList), you will be asked the exact name of your show. This is the show the entry will be linked with. After confirming you should see service id be fill out - if you couldn't find your show on the list or you know the id is wrong then type in the correct ID before continuing.

If the number of shows matches, you will see a number of episodes and you can add the show by pressing ENTER.

If on the other hand you have too little or too many video files in the folder, you will have to respectively enter episodes you have or add episodes manually later. Adding episodes manually means selecting a show you want to add an episode to in the main menu and press E; you will then be asked for a path to the episode.

If everything went well you should see your show on the left, use arrows to select it. To enter episode selection press the RIGHT ARROW or ENTER. Do the same to start watching an episode. To go back press the LEFT ARROW or ESC.

## Configuration
You've probably seen where the configuration file is located during your first setup. 

By default it should be (by plaform)

### Linux
```
$XDG_CONFIG_HOME/lma/Settings.toml
```
or
```
~/.config/lma/Settings.toml
``` 

### Windows
```
%APPDATA%\FakeMichau\lma\Settings.toml
```

Most important is that you don't change the ``service`` as that will cause issues (though going from an external service to "Local" should work)

### Available options for title_sort:
 - ``LocalIdAsc``
 - ``LocalIdDesc``
 - ``TitleAsc``
 - ``TitleDesc``
 - ``ServiceIdAsc``
 - ``ServiceIdDesc``

### Toggle settings
- ``path_instead_of_title`` controls how episodes are names (maybe you feel like title can spoil things?)
- ``autofill_title`` applies to the menu for adding shows where any title will get overridden by a name from the external service
- ``english_show_titles`` can be thought of as equal to ``im_weird``
- ``update_progress_on_start`` will synchronize your progress on app startup, it's disabled by default for now because it can freeze the application for few seconds
- ``relative_episode_score`` makes per episode score be a distance from the show's average episode score making that data more readable

### Headers
Separate headers for shows and episodes. Not all options from shows will work with episodes and vice versa. Respects order in which they are places. Write as a string separated by commas

Options for shows:
 - title

Options for episodes:
 - number
 - title
 - score
 - extra

### Colors
Simply a hex value for a given part. Can be a short one like ``#333``

### Key binds
A mess currently but you can probably figure out how to modify it.
To change ``move_down`` from ``Down`` to ``j`` you would go from:
```
[key_binds]
...
move_down = "Down"
...
```
to
```
[key_binds]
...
[key_binds.move_down]
Char = "j"
```