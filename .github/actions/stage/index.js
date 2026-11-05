const core = require('@actions/core');
const io = require('@actions/io');
const exec = require('@actions/exec');
const {DefaultArtifactClient} = require('@actions/artifact');
const glob = require('@actions/glob');

const path = require('path');
const os = require('os');

async function run() {
    const started_at = Number(process.env.HELIUM_JOB_STARTED_AT) || Math.floor(Date.now() / 1000);

    process.on('SIGINT', function() {
    })
    const from_artifact = core.getBooleanInput('from_artifact', {required: true});
    const upload_final = core.getBooleanInput('upload_final', {required: false});

    const arm = core.getBooleanInput('arm', {required: false})
    console.log(`artifact: ${from_artifact}, upload_final: ${upload_final}`);

    const artifact = new DefaultArtifactClient();
    const artifactName = arm ? 'build-artifact-arm64' : 'build-artifact-x86_64';
    // Set by prepare-environment: the drive with the most free space (D: on hosted runners).
    const root = process.env.WHOZ_ROOT || 'C:\\helium-windows';
    const buildDir = path.join(root, 'build');
    const artifactZip = path.join(root, 'artifacts.zip');
    if (from_artifact && !upload_final) {
        const artifactInfo = await artifact.getArtifact(artifactName);
        await artifact.downloadArtifact(artifactInfo.artifact.id, {path: root});
        await exec.exec('7z', ['x', artifactZip, '-o' + buildDir, '-y']);
        await io.rmRF(artifactZip);
    }

    const args = ['build.py', '--ci', String(started_at)]
    if (process.env.HELIUM_BUILD_JOBS) {
        args.push('-j', process.env.HELIUM_BUILD_JOBS);
    } else if (process.env.RUNNER_ENVIRONMENT === 'github-hosted') {
        const jobs = Math.max(1, Math.min(os.availableParallelism(),
            Math.floor(os.freemem() / (3 * 1024 ** 3))));
        args.push('-j', String(jobs));
    }

    if (arm)
        args.push('--arm')

    if (upload_final) {
        const finalDirectory = core.getInput('final_directory', {required: true});
        const globber = await glob.create(path.join(finalDirectory, 'whoz*'),
            {matchDirectories: false});
        let packageList = await globber.glob();
        const finalArtifactName = arm ? 'whoz-arm64' : 'whoz-x86_64';
        const maxUploadAttempts = 5;
        for (let attempt = 1; attempt <= maxUploadAttempts; ++attempt) {
            try {
                await artifact.deleteArtifact(finalArtifactName);
            } catch (e) {
                // ignored
            }
            try {
                await artifact.uploadArtifact(finalArtifactName, packageList,
                    finalDirectory, { retentionDays: 4, compressionLevel: 0 });
                break;
            } catch (e) {
                console.error(`Upload artifact failed: ${e}`);
                if (attempt === maxUploadAttempts) throw e;
                // Wait 10 seconds between the attempts
                await new Promise(r => setTimeout(r, 10000));
            }
        }

        const { exitCode, stdout } = await exec.getExecOutput('python', [
            'helium-chromium\\utils\\helium_version.py',
            '--print',
            '--tree', 'helium-chromium',
            '--platform-tree', '.'
        ]);

        if (exitCode !== 0) throw `failed getting version: ${exitCode}`;
        core.setOutput('version', stdout.trim());
        core.setOutput('finished', true);
        return;
    }

    await exec.exec('python', ['-m', 'pip', 'install', 'requests', 'Pillow'], {
        cwd: root,
        ignoreReturnCode: true
    });
    const retCode = await exec.exec('python', args, {
        cwd: root,
        ignoreReturnCode: true
    });

    if (retCode > 0 && retCode !== 42) {
        throw `Unexpected return code: ${retCode}`
    }

    core.setOutput('finished', retCode === 0);

    const package_here = retCode === 0 && core.getBooleanInput('package_here') &&
        Date.now() / 1000 - started_at < 5 * 60 * 60;
    core.setOutput('package_here', package_here);

    if (!package_here && core.getBooleanInput('save_artifact')) {
        // 7z exit 1 means warnings (files it could not read), which the old flow accepted too.
        const zipCode = await exec.exec('7z', ['a', '-tzip', artifactZip,
            path.join(buildDir, 'src'), '-mx=3', '-mtc=on'], {ignoreReturnCode: true});
        if (zipCode > 1) throw new Error(`7z failed with exit code ${zipCode}`);
        for (let i = 0; i < 5; ++i) {
            try {
                await artifact.deleteArtifact(artifactName);
            } catch (e) {
                // ignored
            }
            try {
                await artifact.uploadArtifact(artifactName, [artifactZip],
                    root, { retentionDays: 4, compressionLevel: 0 });
                break;
            } catch (e) {
                console.error(`Upload artifact failed: ${e}`);
                if (i === 4) throw e;
                // Wait 10 seconds between the attempts
                await new Promise(r => setTimeout(r, 10000));
            }
        }
    }
}

run().catch(err => core.setFailed(err.message));
