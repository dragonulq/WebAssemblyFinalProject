
#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/binfmts.h>
#include <linux/err.h>
#include <linux/file.h>
#include <linux/init.h>
#include <linux/slab.h>
#include <linux/uaccess.h>

#define WASM_MAGIC      "\0asm"
#define WASM_MAGIC_LEN  4
#define LAUNCHER_PATH   "/usr/libexec/wasm-launcher"

/* ------------------------------------------------------------------ */
/*           core load_binary() implementation for WebAssembly        */
/* ------------------------------------------------------------------ */
static int load_wasm_binary(struct linux_binprm *bprm)
{
        struct file *launcher;
        int ret;

        /*  Is the file really WebAssembly? */
        if (memcmp(bprm->buf, WASM_MAGIC, WASM_MAGIC_LEN))
                return -ENOEXEC;

        /*  Open the fixed launcher ELF                              */
        launcher = open_exec(LAUNCHER_PATH);
        if (IS_ERR(launcher))
                return PTR_ERR(launcher);


        ret = remove_arg_zero(bprm);                     /* drop "./foo.wasm"   */
        if (ret)                                         /*                    */
                goto fail_put;                           /* ← keep errno       */

        ret = copy_string_kernel(bprm->filename, bprm);  /* argv[1]            */
        if (ret < 0)
                goto fail_put;
        bprm->argc++;

        ret = copy_string_kernel(LAUNCHER_PATH, bprm);   /* argv[0]            */
        if (ret < 0)
                goto fail_put;
        bprm->argc++;


        ret = bprm_change_interp(LAUNCHER_PATH, bprm);
        if (ret)
                goto fail_put;


        fput(bprm->file);
        /* install our launcher */
        bprm->file = launcher;
        launcher = NULL;                 /* bprm now owns the reference   */

        /*
         *  Returning -ENOEXEC makes search_binary_handler()
         *    start over with the *new* file.  */
        return -ENOEXEC;

fail_put:
        fput(launcher);
        return ret;
}

/* ------------------------------------------------------------------ */
/*                registration boiler-plate for the module            */
/* ------------------------------------------------------------------ */

static struct linux_binfmt wasm_format = {
        .module       = THIS_MODULE,
        .load_binary  = load_wasm_binary,
        .min_coredump = true,
};

static int __init init_wasm_binfmt(void)
{
        pr_info("binfmt_wasm: registering WebAssembly handler\n");
        register_binfmt(&wasm_format);
        return 0;
}

static void __exit exit_wasm_binfmt(void)
{
        unregister_binfmt(&wasm_format);
        pr_info("binfmt_wasm: unloaded\n");
}

module_init(init_wasm_binfmt);
module_exit(exit_wasm_binfmt);

MODULE_DESCRIPTION("Native WebAssembly binary-format loader");
MODULE_LICENSE("GPL");
